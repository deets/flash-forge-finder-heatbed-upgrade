{{
SPI Laptimer using RX5808/RTC6715

(c) Diez Roggisch
}}

CON
  _clkmode = xtal1 + pll16x
  _xinfreq = 5_000_000

  MPC_DATA_PIN = 17
  MPC_CLK_PIN = 16
  MPC_CS_PIN = 15

  ' for debugging
  DEBUGPIN = 10

  TX_PIN  = 30
  RX_PIN  = 31
  SERIAL_BPS = 115200

VAR

OBJ
  'mcp3008: "MCP3008"
  'fu: "frequency-updater"
  serial: "FullDuplexSerial"

PUB main | h, start_ts, rssi, loopcount
  serial.Start(RX_PIN, TX_PIN, 0, SERIAL_BPS)
  serial.str(@"Start!")
  nl
  repeat
    waitcnt(cnt + clkfreq / 4)
    serial.dec(1000)
    nl

PRI nl
  serial.tx(13)
  serial.tx(10)
