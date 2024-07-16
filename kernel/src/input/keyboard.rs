use pc_keyboard::{ScancodeSet, ScancodeSet1, EventDecoder, HandleControl, DecodedKey};
use pc_keyboard::layouts::Us104Key;

pub struct Keyboard {
    scancode_set: ScancodeSet1,
    event_decoder: EventDecoder<Us104Key>
}

impl Keyboard {
    pub fn new() -> Keyboard {
        Keyboard {
            scancode_set: ScancodeSet1::new(),
            event_decoder: EventDecoder::new(Us104Key, HandleControl::Ignore)
        }
    }

    pub fn read_char(&mut self) -> Option<char> {
        let data = unsafe {
            let status = x86::io::inb(0x64);
            if status & 1 == 0 { return None; }

            x86::io::inb(0x60)
        };

        let event = self.scancode_set.advance_state(data).unwrap();
        if event.is_none() { return None; }
        
        self.event_decoder
            .process_keyevent(event.unwrap())
            .and_then(|x| {
                match x {
                    DecodedKey::Unicode(x) => Some(x),
                    _ => None
                }
            })
    }
}
