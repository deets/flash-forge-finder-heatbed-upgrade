// The voltage divider can have the NTC
// resistor on "top", meaning the voltage
// drop over it is V_ntc = V_in - V_measured. Or
// in the lower leg, then V_ntc = V_measured
pub enum DividerConfiguration
{
    NtcTop,
    NtcBottom,
}

struct Ntc
{
    beta: f32,
    r_o: f32,
    t_o: f32,
}

pub struct Thermistor
{
    v_in: f32,
    r1: f32,
    config: DividerConfiguration,
    ntc: Ntc
}

impl Thermistor {
    pub fn new(v_in: f32, r1: f32, config: DividerConfiguration, beta: f32, r_o: f32, t_o: f32) -> Thermistor
    {
        Thermistor{v_in, r1, config, ntc: Ntc{beta, r_o, t_o}}
    }

    pub fn resistance(&self, v_measured: f32) -> f32
    {
        let v_ntc = match self.config {
            DividerConfiguration::NtcTop => { self.v_in - v_measured }
            DividerConfiguration::NtcBottom => { v_measured }
        };
        v_ntc * self.r1 / (self.v_in - v_ntc)
    }

    pub fn temperature(&self, v_measured: f32) -> f32
    {
        let r_ntc = self.resistance(v_measured);
        let mut steinhart = (r_ntc / self.ntc.r_o).ln() / self.ntc.beta;
        steinhart += 1.0 / (self.ntc.t_o + 273.15);
        (1.0 / steinhart) - 273.15 // Invert, convert to C
    }
}
