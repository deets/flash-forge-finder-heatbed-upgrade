#![allow(unused_imports)]
#![allow(clippy::single_component_path_imports)]
//#![feature(backtrace)]

mod mcp3008;
mod thermistor;
mod pid;
mod events;
mod consts;

use mcp3008::MCP3008;
use thermistor::{Thermistor, DividerConfiguration};
use pid::PIDController;
use events::HeatbedControllerEvent;

use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Condvar, Mutex};
use std::{cell::RefCell, env, sync::atomic::*, sync::Arc, thread, time::*};

use anyhow::bail;

use embedded_svc::mqtt::client::utils::ConnState;
use log::*;

use embedded_hal::adc::OneShot;
use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::digital::v2::{OutputPin, InputPin};

use embedded_svc::eth;
use embedded_svc::eth::{Eth, TransitionalState};
use embedded_svc::httpd::registry::*;
use embedded_svc::httpd::*;
use embedded_svc::io;
use embedded_svc::ipv4;
use embedded_svc::mqtt::client::{Client, Connection, MessageImpl, Publish, QoS};
use embedded_svc::ping::Ping;
use embedded_svc::sys_time::SystemTime;
use embedded_svc::timer::TimerService;
use embedded_svc::timer::*;
use embedded_svc::wifi::*;
use embedded_svc::event_bus::EventBus;
use embedded_svc::event_bus::Postbox;

use esp_idf_svc::eth::*;
use esp_idf_svc::eventloop::*;
use esp_idf_svc::httpd as idf;
use esp_idf_svc::httpd::ServerRegistry;
use esp_idf_svc::mqtt::client::*;
use esp_idf_svc::netif::*;
use esp_idf_svc::nvs::*;
use esp_idf_svc::ping;
use esp_idf_svc::sntp;
use esp_idf_svc::sysloop::*;
use esp_idf_svc::systime::EspSystemTime;
use esp_idf_svc::timer::*;
use esp_idf_svc::wifi::*;

use esp_idf_hal::peripherals;
use esp_idf_sys::EspError;

use esp_idf_hal::adc;
use esp_idf_hal::delay;
use esp_idf_hal::gpio;
use esp_idf_hal::gpio::InterruptType;
use esp_idf_hal::i2c;
use esp_idf_hal::prelude::*;
use esp_idf_hal::spi;

use display_interface_spi::SPIInterfaceNoCS;

use embedded_graphics::mono_font::{ascii::FONT_10X20, MonoTextStyle};
use embedded_graphics::pixelcolor::*;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;
use embedded_graphics::text::*;

use st7789;

use epd_waveshare::{epd4in2::*, graphics::VarDisplay, prelude::*};

#[allow(dead_code)]
#[cfg(not(feature = "qemu"))]
const SSID: &str = env!("RUST_ESP32_STD_DEMO_WIFI_SSID");
#[allow(dead_code)]
#[cfg(not(feature = "qemu"))]
const PASS: &str = env!("RUST_ESP32_STD_DEMO_WIFI_PASS");

// #[cfg(esp32s2)]
// include!(env!("EMBUILD_GENERATED_SYMBOLS_FILE"));

// #[cfg(esp32s2)]
// const ULP: &[u8] = include_bytes!(env!("EMBUILD_GENERATED_BIN_FILE"));

thread_local! {
    static TLS: RefCell<u32> = RefCell::new(13);
}

fn init_esp() -> Result<EspBackgroundEventLoop, EspError> {
    esp_idf_sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    use esp_idf_svc::{netif::EspNetifStack, sysloop::EspSysLoopStack};
    // use esp_idf_svc::nvs::EspDefaultNvs;
    use std::sync::Arc;

    #[allow(unused)]
    let netif_stack = Arc::new(EspNetifStack::new()?);
    #[allow(unused)]
    let sys_loop_stack = Arc::new(EspSysLoopStack::new()?);

    let mut config:BackgroundLoopConfiguration = Default::default();
    config.task_stack_size = 8000;
    Ok(EspBackgroundEventLoop::new(&config)?)
}


use esp_idf_hal::gpio::Gpio21;
use esp_idf_hal::gpio::Gpio34;
use esp_idf_hal::gpio::Gpio35;
use esp_idf_hal::gpio::Gpio36;
use esp_idf_hal::gpio::Gpio37;
use esp_idf_hal::gpio::Gpio38;
use esp_idf_hal::spi::SPI2;
use st7789::ST7789;

type Display = ST7789<SPIInterfaceNoCS<esp_idf_hal::spi::Master<SPI2, Gpio36<esp_idf_hal::gpio::Unknown>, Gpio35<esp_idf_hal::gpio::Unknown>, Gpio21<esp_idf_hal::gpio::Unknown>, Gpio34<esp_idf_hal::gpio::Unknown>>, Gpio37<esp_idf_hal::gpio::Output>>, Gpio38<esp_idf_hal::gpio::Output>>;

fn main() -> Result<()> {
    let mut eventloop = init_esp().expect("Error initializing ESP");
    // Bind the log crate to the ESP Logging facilities

    #[allow(unused)]
    let peripherals = Peripherals::take().unwrap();
    #[allow(unused)]
    let pins = peripherals.pins;

    #[cfg(feature = "ttgo")]
    let mut display = ttgo_hello_world(
        pins.gpio33,
        pins.gpio37,
        pins.gpio38,
        peripherals.spi2,
        pins.gpio36,
        pins.gpio35,
        pins.gpio34,
    )?;

    let adc = MCP3008::new(
        peripherals.spi3,
        pins.gpio12, // clk
        pins.gpio11, // mosi
        pins.gpio13, // miso
        pins.gpio15, // cs
        consts::V_IN,
    )?;
    // See https://github.com/Klipper3d/klipper/issues/1125 for my NTC
    // value assumptionns
    let thermistor = Thermistor::new(
        consts::V_IN,
        4720.0,
        DividerConfiguration::NtcTop,
        3950.0, // beta
        100_000.0, // R_o,
        25.0, // T_o
    );

    let (mut pidcontroller, _) = PIDController::start(adc, thermistor, pins.gpio4, peripherals.rmt.channel0)?;

    // #[allow(clippy::redundant_clone)]
    // #[cfg(not(feature = "qemu"))]
    // #[allow(unused_mut)]
    // let mut wifi = wifi(
    //     netif_stack.clone(),
    //     sys_loop_stack.clone(),r Beuten verbessert werden.
    //     default_nvs.clone(),
    // )?;

    let io0_irq = pins.gpio0.into_input()?;
    let mut io0_eventloop = eventloop.clone();

    let mut io18 = pins.gpio18.into_output()?;
    let _io0_irq = unsafe {
        io0_irq.into_subscribed(
            move || {
                io0_eventloop.post(&HeatbedControllerEvent::IO0, Some(Duration::from_millis(0))).unwrap();
                },
            InterruptType::NegEdge,
        )
    }?;

    let mut state = false;

    // The TTGO board's screen does not start at offset 0x0, and the physical size is 135x240, instead of 240x320
    let top_left = Point::new(52, 40);
    let size = Size::new(135, 240);

    let mut sample_eventloop = eventloop.clone();
    let mut sample_timer = EspTimerService::new()?.timer(move || {
        sample_eventloop.post(&HeatbedControllerEvent::SampleTimer, Some(Duration::from_millis(0))).unwrap();
    })?;
    sample_timer.every(Duration::from_millis(1))?;

    let mut display_eventloop = eventloop.clone();
    let mut display_timer = EspTimerService::new()?.timer(move || {
        display_eventloop.post(&HeatbedControllerEvent::DisplayTimer, Some(Duration::from_millis(0))).unwrap();
    })?;
    display_timer.every(Duration::from_secs(1))?;

    let _subscription = eventloop.subscribe( move |message: &HeatbedControllerEvent| {
        let mut update_display = false;
        match message {
            HeatbedControllerEvent::IO0 => {
                info!("Got message from the event loop");//: {:?}", message.0);
                state = !state;
                if state {
                    io18.set_high().unwrap();
                } else {
                    io18.set_low().unwrap();
                }
                update_display = true;
            },
            HeatbedControllerEvent::DisplayTimer => {
                update_display = true;
            },
            _ => {}
        }

        if update_display {
            let pid_data = pidcontroller.data();
            let power_text = format!(
                "Power: {}", if state { "On" } else { "Off"});
            let adc_text = format!("ADC: {}", pid_data.adc_value);
            let voltage_text = format!("V: {}", pid_data.voltage);
            let resistor_text = format!("Temp: {}", pid_data.temperature);
            led_draw(&power_text, &adc_text, &voltage_text, &resistor_text, &mut display.cropped(&Rectangle::new(top_left, size)))
                .map_err(|e| anyhow::anyhow!("Display error: {:?}", e)).unwrap();
        }
    })?;

    loop {
        // too large a value here triggers the WDT?
        thread::sleep(Duration::from_millis(100));
    }
    Ok(())
}


#[cfg(feature = "ttgo")]
fn ttgo_hello_world(
    backlight: gpio::Gpio33<gpio::Unknown>,
    dc: gpio::Gpio37<gpio::Unknown>,
    rst: gpio::Gpio38<gpio::Unknown>,
    spi: spi::SPI2,
    sclk: gpio::Gpio36<gpio::Unknown>,
    sdo: gpio::Gpio35<gpio::Unknown>,
    cs: gpio::Gpio34<gpio::Unknown>,
) -> Result<Display>
{
    info!("About to initialize the TTGO ST7789 LED driver");

    let config = <spi::config::Config as Default>::default().baudrate(26.MHz().into());

    let mut backlight = backlight.into_output()?;
    backlight.set_high()?;

    let di = SPIInterfaceNoCS::new(
        spi::Master::<spi::SPI2, _, _, _, _>::new(
            spi,
            spi::Pins {
                sclk,
                sdo,
                sdi: Option::<gpio::Gpio21<gpio::Unknown>>::None,
                cs: Some(cs),
            },
            config,
        )?,
        dc.into_output()?,
    );

    let mut display = st7789::ST7789::new(
        di,
        rst.into_output()?,
        // SP7789V is designed to drive 240x320 screens, even though the TTGO physical screen is smaller
        240,
        320,
    );

    display
        .init(&mut delay::Ets)
        .map_err(|e| anyhow::anyhow!("Display error: {:?}", e))?;
    display
        .set_orientation(st7789::Orientation::Portrait)
        .map_err(|e| anyhow::anyhow!("Display error: {:?}", e))?;

    return Ok(display)
}

#[allow(dead_code)]
fn led_draw<D>(power_text: &str, adc_text: &str, voltage_text: &str, resistor_text: &str, display: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget + Dimensions,
    D::Color: From<Rgb565>,
{
    display.clear(Rgb565::BLACK.into())?;

    Rectangle::new(display.bounding_box().top_left, display.bounding_box().size)
        .into_styled(
            PrimitiveStyleBuilder::new()
                .fill_color(Rgb565::BLUE.into())
                .stroke_color(Rgb565::YELLOW.into())
                .stroke_width(1)
                .build(),
        )
        .draw(display)?;
    let pos = Point::new(10, (display.bounding_box().size.height - 10) as i32 / 2);
    Text::new(
        power_text,
        pos,
        MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE.into()),
    ).draw(display)?;
    let offset = Point::new(0, 24);
    Text::new(
        adc_text,
        pos + offset ,
        MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE.into()),
    ).draw(display)?;
    Text::new(
        voltage_text,
        pos + offset * 2 ,
        MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE.into()),
    ).draw(display)?;
    Text::new(
        resistor_text,
        pos + offset * 3 ,
        MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE.into()),
    ).draw(display)?;

    Ok(())
}


#[cfg(not(feature = "qemu"))]
#[allow(dead_code)]
fn wifi(
    netif_stack: Arc<EspNetifStack>,
    sys_loop_stack: Arc<EspSysLoopStack>,
    default_nvs: Arc<EspDefaultNvs>,
) -> Result<Box<EspWifi>> {
    let mut wifi = Box::new(EspWifi::new(netif_stack, sys_loop_stack, default_nvs)?);

    info!("Wifi created, about to scan");

    let ap_infos = wifi.scan()?;

    let ours = ap_infos.into_iter().find(|a| a.ssid == SSID);

    let channel = if let Some(ours) = ours {
        info!(
            "Found configured access point {} on channel {}",
            SSID, ours.channel
        );
        Some(ours.channel)
    } else {
        info!(
            "Configured access point {} not found during scanning, will go with unknown channel",
            SSID
        );
        None
    };

    wifi.set_configuration(&Configuration::Mixed(
        ClientConfiguration {
            ssid: SSID.into(),
            password: PASS.into(),
            channel,
            ..Default::default()
        },
        AccessPointConfiguration {
            ssid: "aptest".into(),
            channel: channel.unwrap_or(1),
            ..Default::default()
        },
    ))?;

    info!("Wifi configuration set, about to get status");

    wifi.wait_status_with_timeout(Duration::from_secs(20), |status| !status.is_transitional())
        .map_err(|e| anyhow::anyhow!("Unexpected Wifi status: {:?}", e))?;

    let status = wifi.get_status();

    if let Status(
        ClientStatus::Started(ClientConnectionStatus::Connected(ClientIpStatus::Done(ip_settings))),
        ApStatus::Started(ApIpStatus::Done),
    ) = status
    {
        info!("Wifi connected");
    } else {
        bail!("Unexpected Wifi status: {:?}", status);
    }

    Ok(wifi)
}
