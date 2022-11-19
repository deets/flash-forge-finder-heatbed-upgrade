// The voltage divider can have the NTC
// resistor on "top", meaning the voltage
// drop over it is V_ntc = V_in - V_measured. Or
// in the lower leg, then V_ntc = V_measured
pub enum DividerConfiguration
{
    NtcTop,
    NtcBottom,
}

pub struct Thermistor
{
    v_in: f32,
    r1: f32,
    config: DividerConfiguration,
}

impl Thermistor {
    pub fn new(v_in: f32, r1: f32, config: DividerConfiguration) -> Thermistor
    {
        Thermistor{v_in, r1, config}
    }

    pub fn resistance(&self, v_measured: f32) -> f32
    {
        let v_ntc = match self.config {
            DividerConfiguration::NtcTop => { self.v_in - v_measured }
            DividerConfiguration::NtcBottom => { v_measured }
        };
        v_ntc * self.r1 / (self.v_in - v_ntc)
    }
}
