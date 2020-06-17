use std::sync::{Arc, Mutex};
use std::marker::PhantomData;

use druid::{Widget, WidgetExt, lens::{self, LensExt}, UnitPoint, Env, EventCtx, Command, Selector, Data, Event, UpdateCtx};
use druid::widget::{List, Flex, Label, Scroll, Container, CrossAxisAlignment, SizedBox, ViewSwitcher, FillStrat, Either, Controller};
use druid::im::Vector;

use druid_icon::Icon;

use kiro_synth_core::float::Float;
use kiro_synth_engine::program::{SourceRef, ParamRef};

use crate::synth::SynthClient;
use crate::ui::{GREY_83, KNOB_MODULATION, GREY_74, KNOB_VALUE, KNOB_WEIGHT};
use crate::ui::model::SynthModel;
use crate::ui::widgets::knob::{Knob, KnobData};
use crate::ui::model::modulations::{Group, Modulation, View, Modulations, ConfigMode, Reference};
use crate::ui::view::build_static_tabs;
use crate::ui::icons;


pub const START_MODULATIONS_CONFIG: Selector<SourceRef> = Selector::new("synth.modulation.start-config");
pub const CHANGE_MODULATIONS_CONFIG: Selector<(SourceRef, ParamRef, f64)> = Selector::new("synth.modulation.change-config");
pub const STOP_MODULATIONS_CONFIG: Selector<SourceRef> = Selector::new("synth.modulation.stop-config");


pub struct ModulationController<T: Data> {
  _phantom: PhantomData<T>
}

impl<T: Data> ModulationController<T> {
  pub fn new() -> Self {
    ModulationController {
      _phantom: PhantomData
    }
  }
}

impl<W: Widget<SynthModel>> Controller<SynthModel, W> for ModulationController<SynthModel> {
  fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut SynthModel, env: &Env) {
    match event {
      Event::Command(command) if command.is(START_MODULATIONS_CONFIG) => {
        if let Some(source_ref) = command.get::<SourceRef>(START_MODULATIONS_CONFIG) {
          data.start_modulations_config(*source_ref);
        }
      }
      Event::Command(command) if command.is(CHANGE_MODULATIONS_CONFIG) => {
        if let Some((source_ref, param_ref, weight)) = command.get::<(SourceRef, ParamRef, f64)>(CHANGE_MODULATIONS_CONFIG) {
          data.change_modulations_config(*source_ref, *param_ref, *weight);
        }
      }
      Event::Command(command) if command.is(STOP_MODULATIONS_CONFIG) => {
        if let Some(source_ref) = command.get::<SourceRef>(STOP_MODULATIONS_CONFIG) {
          data.stop_modulations_config(*source_ref);
        }
      }
      _ => {}
    }

    child.event(ctx, event, data, env);
  }
}

pub struct ModulationsView;

impl ModulationsView {
  pub fn new<F: Float + 'static>(_synth_client: Arc<Mutex<SynthClient<F>>>) -> impl Widget<SynthModel> {

    let views = vec![
      View::GroupBySource,
      View::GroupByParam,
    ];
    let tabs = build_static_tabs(views, Self::build_tab)
        .lens(Modulations::view);

    let body = ViewSwitcher::new(
      |data: &Modulations, _: &Env| data.view,
      |view: &View, _data: &Modulations, _: &Env| {
        match view {
          View::GroupBySource => Box::new(Self::build_modulations_list()),
          View::GroupByParam => Box::new(Self::build_modulations_list()),
        }
      }
    );

    let body = Container::new(body)
        .rounded(2.0)
        .border(GREY_83, 2.0)
        .padding((4.0, 0.0, 4.0, 4.0))
        .expand_height();

    Flex::column()
        .with_child(tabs.padding((4.0, 4.0, 0.0, 0.0)))
        .with_flex_child(body, 1.0)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .lens(SynthModel::modulations)
  }

  fn build_tab(_index: usize, data: &View) -> impl Widget<View> {
    let icon = match data {
      View::GroupBySource => &icons::MODULATION_SOURCES,
      View::GroupByParam => &icons::MODULATION_PARAMS,
    };

    Icon::new(icon)
        .fix_height(12.0)
        .center()
        .padding((6.0, 4.0, 4.0, 2.0))
  }

  fn build_modulations_list() -> impl Widget<Modulations> {
    let list = List::new(|| {
      Flex::column()
          .with_child(Self::build_group())
          .with_child(Self::build_modulation_knobs())
    });

    Scroll::new(list.padding((4.0, 0.0))).vertical()
  }

  fn build_group() -> impl Widget<Group> {

    let group_icon = Either::new(
      |data: &Group, _: &Env| match data.reference {
        Reference::Source(_) => true,
        Reference::Param(_) => false,
      },
      Icon::new(&icons::MODULATION_SOURCE)
          .fill_strategy(FillStrat::ScaleDown)
          .fix_height(9.0),
      Icon::new(&icons::MODULATION_PARAM)
          .fill_strategy(FillStrat::ScaleDown)
          .fix_height(9.0),
    );

    let name = Label::new(|data: &Group, _env: &_| data.name.clone())
        .padding((3.0, 3.0))
        .expand_width()
        .height(20.0);

    let config_mode = ViewSwitcher::new(
      |data: &Group, _: &Env| data.config_mode,
      |config_mode: &ConfigMode, data: &Group, _: &Env| {
        if let Reference::Source(source_ref) = data.reference {
          match config_mode {
            ConfigMode::Ready => {
              Icon::new(&icons::MODULATION_ARROW)
                  .fill_strategy(FillStrat::ScaleDown)
                  .fix_height(10.0)
                  .on_click(move |ctx: &mut EventCtx, _data: &mut Group, _: &Env| {
                    let command = Command::new(START_MODULATIONS_CONFIG, source_ref);
                    ctx.submit_command(command, None);
                  })
                  .boxed()
            }
            ConfigMode::Ongoing => {
              Icon::new(&icons::MODULATION_ARROW)
                  .color(KNOB_WEIGHT)
                  .fill_strategy(FillStrat::ScaleDown)
                  .fix_height(10.0)
                  .on_click(move |ctx: &mut EventCtx, _data: &mut Group, _: &Env| {
                    let command = Command::new(STOP_MODULATIONS_CONFIG, source_ref);
                    ctx.submit_command(command, None);
                  })
                  .boxed()
            }
            ConfigMode::Disabled => {
              Icon::new(&icons::MODULATION_ARROW)
                  .color(GREY_74)
                  .fill_strategy(FillStrat::ScaleDown)
                  .fix_height(10.0)
                  .boxed()
            }
          }
        }
        else {
          SizedBox::empty().fix_height(10.0).boxed()
        }
      },
    ).padding((0.0, 0.0, 2.0, 0.0));

    Flex::row()
        .with_child(group_icon)
        .with_flex_child(name, 1.0)
        .with_child(config_mode)
  }

  fn build_modulation_knobs() -> impl Widget<Group> {
    List::new(|| Self::build_modulation_knob())
      .lens(lens::Id.map(
        |data: &Group| data.modulations.clone(),
        |data: &mut Group, list_data: Vector<Modulation>| data.modulations = list_data,
      ))
  }

  fn build_modulation_knob() -> impl Widget<Modulation> {
    let name = Label::new(|data: &Modulation, _env: &_| data.name.clone())
        .align_vertical(UnitPoint::new(0.0, 0.5))
        .fix_height(19.0);

    let value_fn = move |data: &Modulation, _env: &_| {
      let step = data.step.max(0.001);
      let precision = (-step.log10().ceil()).max(0.0).min(3.0) as usize;
      let value = (data.amount / step).round() * step;
      format!("{:.*}", precision, value)
    };

    let value = Label::new(value_fn)
        .align_vertical(UnitPoint::new(0.0, 0.5))
        .fix_height(19.0);

    let name_and_value = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(name)
        .with_child(value)
        .expand_width();

    let callback = move |ctx: &mut UpdateCtx, data: &KnobData<Modulation>| {
      let source_ref = data.context.source_ref;
      let param_ref = data.context.param_ref;
      data.context.synth_client
          .send_modulation_amount(source_ref, param_ref, data.value as f32).unwrap();
      let payload = (source_ref, param_ref, data.value);
      let command = Command::new(CHANGE_MODULATIONS_CONFIG, payload);
      ctx.submit_command(command, None)
    };

    let knob = Knob::new(callback)
        .padding(4.0)
        .center()
        .fix_size(38.0, 38.0)
        .lens(lens::Id.map(
          |data: &Modulation| {
            KnobData::new(data.origin, data.min, data.max, data.step, data.amount, data.clone())
          },
          |data: &mut Modulation, knob_data: KnobData<Modulation>| {
            data.amount = knob_data.value
          }
        ));

    Flex::row()
        .with_child(knob)
        .with_flex_child(name_and_value, 1.0)
  }
}
