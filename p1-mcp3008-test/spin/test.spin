{{
SPI Laptimer using RX5808/RTC6715

(c) Diez Roggisch
}}

CON
  _clkmode = xtal1 + pll16x
  _xinfreq = 5_000_000

  MPC_DATA_PIN = 23
  MPC_CLK_PIN = 25
  MPC_CS_PIN = 27

  ' for debugging
  DEBUGPIN = 10

  TX_PIN  = 30
  RX_PIN  = 31
  SERIAL_BPS = 115200

VAR

OBJ
  mcp3008: "MCP3008"
  serial: "FullDuplexSerial"

PUB main | h
  serial.Start(RX_PIN, TX_PIN, 0, SERIAL_BPS)
  serial.str(@"Start!")
  mcp3008.start(MPC_DATA_PIN, MPC_CLK_PIN, MPC_CS_PIN, (|< 1) - 1)
  nl
  repeat
    waitcnt(cnt + clkfreq / 4)
    h := mcp3008.in(0)
    serial.dec(h)
    nl

PRI nl
  serial.tx(13)
  serial.tx(10)
