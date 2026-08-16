# Pad captures

Raw HID reports from a real controller, one press per line.

`poe-wayfinder.exe --pad-walkthrough dualsense-usb.hex` writes one. It names a
button, waits for a press, records the report and moves on. Copy the file here
and `pad_capture_replay.rs` replays it through the parser on every test run,
with no pad and no Windows.

Format:

```
# product 0x0ce6
# report_len 64
Square 01 80 80 80 80 00 00 00 18 00 ...
```

The header names the product so the parser picks the right offsets. Each other
line is a button label then the report bytes in hex.

One file per pad and per transport, because the offsets differ:
`dualsense-usb.hex`, `dualsense-bt.hex`, `dualshock4-usb.hex`.

A capture is evidence. Never hand edit one. If a line is wrong the parser is
wrong, or the capture was taken with two buttons held.
