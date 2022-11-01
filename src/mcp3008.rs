use esp_idf_hal::spi;
use esp_idf_hal::gpio;
use esp_idf_hal::prelude::*;
use esp_idf_sys::EspError;

pub struct MCP3008
{
    spi: spi::Master<spi::SPI3, gpio::Gpio12<gpio::Unknown>, gpio::Gpio11<gpio::Unknown>>
}


impl MCP3008 {
    pub fn new(spi: spi::SPI3,
               clk: gpio::Gpio12<gpio::Unknown>,
               si: gpio::Gpio11<gpio::Unknown>,
               so: gpio::Gpio13<gpio::Unknown>,
               cs: gpio::Gpio10<gpio::Unknown>
    ) -> Result<MCP3008, EspError>
    {
        let config = <spi::config::Config as Default>::default().baudrate(5.MHz().into());

        let di = spi::Master::<spi::SPI3, _, _, _, _>::new(
            spi,
            spi::Pins {
                sclk: clk,
                sdo: si,
                sdi: Some(so),
                cs: Some(cs)
            },
            config
            )?;
        Ok(MCP3008{spi: di})
    }

    pub fn read(&self, channel: u8) -> u32 {
        0
    }
}
