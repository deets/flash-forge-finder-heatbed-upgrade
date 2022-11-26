use std::thread::JoinHandle;
use std::sync::{Arc, Mutex};
use std::{thread, time::*};
use esp_idf_svc::eventloop::*;
use embedded_svc::event_bus::EventBus;
use embedded_svc::event_bus::Postbox;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::gpio::Pins;
use anyhow::Result;
use biquad::*;


use crate::events::HeatbedControllerEvent;
use crate::consts::V_IN;
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

impl PIDController {
    pub fn start<ADC:SingleChannelRead + Send + 'static>(mut adc: ADC, thermistor: Thermistor)
                                                         -> Result<(PIDController, JoinHandle<()>)>
    {
        let temperature = Arc::new(Mutex::new(
            PIDData{ temperature: 0.0, voltage: 0.0, adc_value: 0}
        ));
        let pid_temperature = Arc::clone(&temperature);

        let r = thread::Builder::new().stack_size(4096).spawn(move || {
            let mut adc_filter = create_adc_filter();

            loop {
                let adc_reading = adc.read(0).expect("SPI broke");
                let v_r1 = adc_filter.run(adc_reading.voltage);
                let temp = thermistor.temperature(v_r1);
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