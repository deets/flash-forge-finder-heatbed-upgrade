use std::thread::JoinHandle;
use std::sync::{Arc, Mutex};
use std::{thread, time::*};
use esp_idf_svc::eventloop::*;
use embedded_svc::event_bus::EventBus;
use embedded_svc::event_bus::Postbox;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::gpio::Pins;
use esp_idf_hal::gpio::OutputPin;
use anyhow::Result;
use biquad::*;
use esp_idf_hal::rmt::config::{Loop, TransmitConfig};
use esp_idf_hal::rmt::*;
use embedded_hal::PwmPin;
use num::clamp;

use crate::events::HeatbedControllerEvent;
use crate::consts::{V_IN, RMT_CLOCK_DIVIDER, RMT_DUTY_CYCLE};
use crate::mcp3008::SingleChannelRead;
use crate::thermistor::{Thermistor, DividerConfiguration};

#[derive(Copy, Clone)]
pub struct PIDData
{
    pub temperature: f32,
    pub voltage: f32,
    pub adc_value: u32
}

pub struct PIDController
{
    temperature: Arc<Mutex<PIDData>>
}

fn create_adc_filter() -> DirectForm1<f32>
{
    // Cutoff and sampling frequencies
    let f0 = 1.hz();
    let fs = 100.hz();
    let coeffs = Coefficients::<f32>::from_params(Type::LowPass, fs, f0, Q_BUTTERWORTH_F32).unwrap();
    DirectForm1::<f32>::new(coeffs)
}

fn create_pwm<PwmPin: OutputPin, RmtChannel: HwChannel>(pwm_pin: PwmPin, rmt_channel: RmtChannel) -> Result<Transmit<PwmPin, RmtChannel>>
{
    let config = TransmitConfig::new().clock_divider(RMT_CLOCK_DIVIDER).looping(Loop::Endless);
    Ok(Transmit::new(pwm_pin, rmt_channel, &config)?)
}

fn set_duty_cycle<PwmPin: OutputPin, RmtChannel: HwChannel>(pwm: &mut Transmit<PwmPin, RmtChannel>, duty_cycle: f32)
{
    let duty_cycle = clamp(duty_cycle, 0.0, 1.0);
    let ticks_hz = pwm.counter_clock().unwrap();
    let period = u32::from(ticks_hz / 1000); // These many ticks for one period
    let duty_cycle = period * (duty_cycle * 1000.0) as u32 / 1000;
    let rest_cycle = period - duty_cycle;
    let mut high = Pulse::new(PinState::High, PulseTicks::max());
    let mut low = Pulse::new(PinState::Low, PulseTicks::max());
    if duty_cycle == 0 {
        high = low;
    } else if rest_cycle == 0 {
        low = high;
    } else {
        high = Pulse::new(PinState::High, PulseTicks::new(duty_cycle as u16).unwrap());
        low = Pulse::new(PinState::Low, PulseTicks::new(rest_cycle as u16).unwrap());
    }
    let mut signal = FixedLengthSignal::<1>::new();
    signal.set(0, &(low, high)).unwrap();
    pwm.start(signal).unwrap();
}

impl PIDController {
    pub fn start<ADC:SingleChannelRead + Send + 'static, PwmPin: OutputPin + 'static, RmtChannel: HwChannel + Send + 'static>(
        mut adc: ADC,
        thermistor: Thermistor,
        pwm_pin: PwmPin,
        rmt_channel: RmtChannel
    ) -> Result<(PIDController, JoinHandle<()>)>
    {
        let temperature = Arc::new(Mutex::new(
            PIDData{ temperature: 0.0, voltage: 0.0, adc_value: 0}
        ));
        let pid_temperature = Arc::clone(&temperature);

        let r = thread::Builder::new().stack_size(4096).spawn(move || {
            let mut adc_filter = create_adc_filter();
            let mut pwm = create_pwm(pwm_pin, rmt_channel).unwrap();
            let mut duty_cycle = 0.0;

            loop {
                let adc_reading = adc.read(0).expect("SPI broke");
                let v_r1 = adc_filter.run(adc_reading.voltage);
                let temp = thermistor.temperature(v_r1);
                duty_cycle += 0.01;
                set_duty_cycle(&mut pwm, duty_cycle);

                {
                    let mut t = pid_temperature.lock().unwrap();
                    t.temperature = temp;
                    t.adc_value = adc_reading.raw;
                    t.voltage = v_r1;
                }
                thread::sleep(Duration::from_millis(10));
            }
        })?;
        Ok((PIDController{ temperature }, r))
    }

    pub fn data(&mut self) -> PIDData
    {
        *self.temperature.lock().unwrap()
    }
}
