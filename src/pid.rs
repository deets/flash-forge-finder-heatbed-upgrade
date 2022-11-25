use std::thread::JoinHandle;
use std::{thread, time::*};
use esp_idf_svc::eventloop::*;
use embedded_svc::event_bus::EventBus;
use embedded_svc::event_bus::Postbox;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::gpio::Pins;
use anyhow::Result;

use crate::events::HeatbedControllerEvent;
use crate::consts::V_IN;
use crate::mcp3008::SingleChannelRead;
use crate::thermistor::{Thermistor, DividerConfiguration};

pub struct PIDController<'a, ADC:SingleChannelRead>
{
    adc: &'a ADC
}


impl<ADC:SingleChannelRead + Send + 'static> PIDController<'_,ADC> {
    pub fn start(mut adc: ADC) -> Result<JoinHandle<()>>
    {
        // See https://github.com/Klipper3d/klipper/issues/1125 for my NTC
        // value assumptionns
        let thermistor = Thermistor::new(
            V_IN,
            4720.0,
            DividerConfiguration::NtcTop,
            3950.0, // beta
            100_000.0, // R_o,
            25.0, // T_o
        );

        let r = thread::Builder::new().stack_size(4096).spawn(move || {
            loop {
                let adc_value = adc.read(0).expect("SPI broke");
                println!("Background thread working: {}", adc_value.voltage);
                thread::sleep(Duration::from_millis(100));
            }
        })?;
        Ok(r)
    }
}
