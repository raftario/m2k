# m2k

Translate MIDI note on and off messages to Windows key down and up inputs.

## Configuration

By default C3 is mapped to spacebar and C4 through G4 are mapped to `c` through `g`. Pass a config file as the first argument (or drag it onto the executable) to override these defaults with custom mappings.

```toml
# C3 -> spacebar
[[mapping]]
# http://www.music.mcgill.ca/~ich/classes/mumt306/StandardMIDIfileformat.html#BMA1_3
note = 48
# https://learn.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes
key = 0x20

# C4 -> C
[[mapping]]
note = 60
key = 0x43
```

## Why ?

Fortnite.

## How's the latency ?

Very good.

## Can you add support for-

No.
