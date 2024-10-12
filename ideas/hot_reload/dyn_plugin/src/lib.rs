use hot_reload::{plugins::PluginGlue, App, Plugin};

pub struct AnswerPlugin;

impl Plugin for AnswerPlugin {
    fn build(&self, app: &mut App) {
        app.register_number(42);
    }
}

#[no_mangle]
pub extern "C" fn __hot_reload_plugin_build_glue() -> *const PluginGlue {
    use std::{
        any::{Any, TypeId},
        mem::MaybeUninit,
    };

    fn build(app: &mut App) {
        eprintln!("Hello, from dyn_plugin!");
        AnswerPlugin.build(app)
    }

    static mut GLUE: MaybeUninit<PluginGlue> = MaybeUninit::zeroed();

    let glue = PluginGlue {
        unit_type_id: ().type_id(),
        plugin_dyn_id: TypeId::of::<AnswerPlugin>().into(),
        build,
    };

    unsafe { GLUE.write(glue) }
}
