use std::{
    env,
    mem::size_of,
    process,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};

use dialoguer::{theme::ColorfulTheme, Confirm, FuzzySelect};
use miette::{Report, Result};
use windows::{
    core::ComInterface,
    Devices::{
        Enumeration::DeviceInformation,
        Midi::{
            IMidiMessage, MidiInPort, MidiMessageReceivedEventArgs, MidiMessageType,
            MidiNoteOffMessage, MidiNoteOnMessage,
        },
    },
    Foundation::TypedEventHandler,
    Win32::UI::{
        Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
            KEYEVENTF_KEYUP,
        },
        WindowsAndMessaging::GetMessageExtraInfo,
    },
};

use crate::mappings::{Mappings, MappingsError};

mod mappings;

fn main() -> Result<()> {
    let run = with_shutdown();
    let mappings = read_mappings()?;
    let (device, debug) = read_options()?;

    run(mappings, device, debug).map_err(Into::into)
}

fn handle_midi_message(
    message: &IMidiMessage,
    mappings: &Mappings,
    debug: bool,
) -> Result<(), windows::core::Error> {
    let ty = message.Type()?;

    let (note, ty) = match ty {
        MidiMessageType::NoteOn => {
            let message: MidiNoteOnMessage = message.cast()?;
            let note = message.Note()?;

            if debug {
                println!("{note}");
            }

            (note, KEYBD_EVENT_FLAGS(0))
        }
        MidiMessageType::NoteOff => {
            let message: MidiNoteOffMessage = message.cast()?;
            let note = message.Note()?;
            (note, KEYEVENTF_KEYUP)
        }
        _ => return Ok(()),
    };

    let key = match mappings.get(note) {
        Some(key) => key,
        None => return Ok(()),
    };

    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
                wScan: 0,
                dwFlags: ty,
                time: 0,
                dwExtraInfo: unsafe { GetMessageExtraInfo().0 as usize },
            },
        },
    };
    let sent = unsafe { SendInput(&[input], size_of::<INPUT>() as i32) };

    if sent == 1 {
        Ok(())
    } else {
        Err(windows::core::Error::from_win32())
    }
}

fn with_shutdown() -> impl Fn(Mappings, MidiInPort, bool) -> Result<(), Error> {
    let should_exit = Arc::new(AtomicBool::new(false));

    ctrlc::set_handler({
        let should_exit = should_exit.clone();
        let main_thread = thread::current();
        move || {
            if should_exit.swap(true, Ordering::AcqRel) {
                process::exit(1);
            }
            main_thread.unpark();
        }
    })
    .unwrap();

    move |mappings, device, debug| {
        device.MessageReceived(
            &TypedEventHandler::<MidiInPort, MidiMessageReceivedEventArgs>::new(move |_, event| {
                let message = match event.as_ref() {
                    Some(event) => event.Message()?,
                    None => return Ok(()),
                };

                if let Err(error) = handle_midi_message(&message, &mappings, debug) {
                    report_error(error);
                }
                Ok(())
            }),
        )?;

        while !should_exit.load(Ordering::Acquire) {
            thread::park();
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum Error {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Config(MappingsError),

    #[error("No MIDI devices found")]
    #[diagnostic(code(devices))]
    NoMidiDevices,

    #[error("Windows error")]
    #[diagnostic(code(os))]
    Windows(#[from] windows::core::Error),

    #[error("IO error")]
    #[diagnostic(code(io))]
    Io(#[from] std::io::Error),

    #[error("Cancellation signal error")]
    #[diagnostic(code(signal))]
    Cancellation(#[from] ctrlc::Error),
}

fn read_mappings() -> Result<Mappings, Error> {
    let path = env::args().nth(1);
    if let Some(path) = path {
        Mappings::from_file(path)
    } else {
        Ok(Mappings::hardcoded())
    }
}

fn read_options() -> Result<(MidiInPort, bool), Error> {
    let midi_device_selector = MidiInPort::GetDeviceSelector()?;
    let midi_devices = DeviceInformation::FindAllAsyncAqsFilter(&midi_device_selector)?.get()?;

    let (midi_names, midi_ids) = midi_devices
        .into_iter()
        .filter_map(|device| {
            let name = device.Name().ok()?;
            let id = device.Id().ok()?;
            Some((name, id))
        })
        .fold((vec![], vec![]), |(mut names, mut ids), (name, id)| {
            names.push(name);
            ids.push(id);
            (names, ids)
        });

    let theme = ColorfulTheme::default();

    let selected = match midi_ids.len() {
        0 => return Err(Error::NoMidiDevices),
        1 => 0,
        _ => FuzzySelect::with_theme(&theme)
            .with_prompt("MIDI device")
            .items(&midi_names)
            .interact()
            .unwrap(),
    };
    let device_id = &midi_ids[selected];

    let debug = Confirm::with_theme(&theme)
        .with_prompt("Debug note IDs")
        .default(false)
        .interact()
        .unwrap();

    let device = MidiInPort::FromIdAsync(device_id)?.get()?;

    Ok((device, debug))
}

#[cold]
fn report_error(error: impl Into<Error>) {
    let report = Report::from(error.into());
    eprintln!("Error: {report}");
}
