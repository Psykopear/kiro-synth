use heapless::Vec;
use heapless::consts;
use typenum::marker_traits::Unsigned;
use ringbuf::Consumer;

use crate::float::Float;
use crate::program::Program;
use crate::voice::Voice;
use crate::event::{Message, Event};
use crate::globals::SynthGlobals;

type MaxVoices = consts::U32;

pub struct Synth<'a, F: Float> {
  _sample_rate: F,
  events: Consumer<Event<F>>,
  program: Program<'a, F>,
  globals: SynthGlobals<F>,
  voices: Vec<Voice<F>, MaxVoices>,
  active_voices: Vec<usize, MaxVoices>,
  free_voices: Vec<usize, MaxVoices>,
}

impl<'a, F: Float> Synth<'a, F> {

  pub fn new(sample_rate: F,
             events: Consumer<Event<F>>,
             program: Program<'a, F>,
             globals: SynthGlobals<F>) -> Self {

    let mut voices: Vec<Voice<F>, MaxVoices> = Vec::new();
    let mut free_voices: Vec<usize, MaxVoices> = Vec::new();
    for index in 0..MaxVoices::to_usize() {
      drop(voices.push(Voice::new(sample_rate, &program)));
      drop(free_voices.push(MaxVoices::to_usize() - index - 1));
    }

    Synth {
      _sample_rate: sample_rate,
      events,
      program,
      globals,
      voices,
      active_voices: Vec::new(),
      free_voices,
    }
  }

  pub fn prepare(&mut self) {
    while let Some(Event { timestamp: _, message }) = self.events.pop() {
      match message {
        Message::NoteOn { key, velocity } => {
          self.note_on(key, velocity)
        },
        Message::NoteOff { key, velocity } => {
          self.note_off(key, velocity)
        },
        Message::Param { param_ref, value } => {
          if let Some((_, param)) = self.program.get_param_mut(param_ref) {
            println!("{} = {:?}", param.id, value);
            param.signal.set(value)
          }
        },
        Message::ParamChange { param_ref, change } => {
          if let Some((_, param)) = self.program.get_param_mut(param_ref) {
            let value = param.signal.get() + change;
            let value = value.min(param.values.max).max(param.values.min);
            println!("{} = {:?}", param.id, value);
            param.signal.set(value);
          }
        },
        Message::ModulationAmount { param_ref, source_ref, amount } => {
          if let Some(source) = self.program.get_source(source_ref) {
            let source_id = source.id;
            if let Some((_, param)) = self.program.get_param_mut(param_ref) {
              println!("{} -> {} {:?}", source_id, param.id, amount);
              param.modulators.iter_mut()
                  .find(|m| m.source == source_ref) // TODO use a HashMap ?
                  .map(|m| m.amount = amount);
            }
          }
        }
      }
    }
  }

  fn note_on(&mut self, key: u8, velocity: F) {
    if let Some(index) = self.allocate_voice(key, velocity) {
      drop(self.active_voices.push(index));
      self.voices[index].note_on(&mut self.program, key, velocity);
      println!("{:?}", self.active_voices);
    }
  }

  fn note_off(&mut self, key: u8, _velocity: F) {
    for active_voice_index in 0..self.active_voices.len() {
      let voice_index = self.active_voices[active_voice_index];
      let voice = &mut self.voices[voice_index];
      if voice.get_key(&self.program) == key {
        voice.note_off(&self.program)
      }
    }
  }

  fn allocate_voice(&mut self, _key: u8, _velocity: F) -> Option<usize> {
    self.free_voices.pop()
  }

  pub fn process(&mut self) -> (F, F) {
    let (mut left, mut right) = (F::zero(), F::zero());

    let mut freed_voices = false;
    let mut active_voice_index = 0;
    while active_voice_index < self.active_voices.len() {
      let voice_index = self.active_voices[active_voice_index];
      let voice = &mut self.voices[voice_index];

      voice.process(&mut self.program, &self.globals);
      let (voice_left, voice_right) = voice.output(&self.program);
      left = left + voice_left;
      right = right + voice_right;

      if voice.is_off(&self.program) {
        self.active_voices.swap_remove(active_voice_index);
        drop(self.free_voices.push(voice_index));
        freed_voices = true;
      }
      else {
        active_voice_index += 1;
      }
    }

    if freed_voices {
      println!("{:?}", self.active_voices);
    }

    self.program.update_params();

    (left, right)
  }
}
