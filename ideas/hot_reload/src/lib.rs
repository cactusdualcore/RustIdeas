use dynamic::DynId;
use std::any::Any;

#[derive(Debug, Default)]
pub struct App {
    plugins: Vec<DynId>,
    numbers: Vec<i32>,
}

impl App {
    pub fn register_number(&mut self, num: i32) -> &mut Self {
        self.numbers.push(num);
        self
    }

    pub fn add_plugin<P: Plugin>(&mut self, plugin: P) -> &mut Self {
        plugin.build(self);

        let dyn_id = plugin.dyn_id();
        self.plugins.push(dyn_id);

        self
    }
}

pub trait Plugin: Any {
    fn dyn_id(&self) -> DynId {
        self.type_id().into()
    }

    fn build(&self, app: &mut App);
}

mod dynamic {
    use std::any::TypeId;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum DynId {
        Typed(TypeId),
        Extern(u64),
    }

    impl From<TypeId> for DynId {
        fn from(tid: TypeId) -> Self {
            Self::Typed(tid)
        }
    }

    impl DynId {
        pub fn new_extern() -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            Self::Extern(COUNTER.fetch_add(1, Ordering::Relaxed))
        }

        pub fn to_typeid(&self) -> Option<TypeId> {
            match self {
                Self::Typed(tid) => Some(*tid),
                Self::Extern(_) => None,
            }
        }
    }
}

pub mod plugins {
    use libloading::Library;
    use std::{
        any::{Any, TypeId},
        cell::OnceCell,
        path::{Path, PathBuf},
    };

    use crate::{dynamic::DynId, App, Plugin};

    type BuildFn = unsafe fn(&mut App);

    #[repr(C)]
    pub struct PluginGlue {
        pub unit_type_id: TypeId,
        pub plugin_dyn_id: DynId,
        pub build: BuildFn,
    }

    #[derive(Debug)]
    struct LibraryFns {
        build_fn: BuildFn,
        // this should be dropped last, so it needs to be the last struct field
        _lib: Library,
    }

    #[derive(Debug)]
    pub struct DynPlugin {
        name: String,
        library_path: PathBuf,
        plugin_dyn_id: OnceCell<DynId>,
        library_fns: OnceCell<LibraryFns>,
    }

    impl DynPlugin {
        pub fn from_library_path(path: &Path) -> Self {
            let path = path.to_owned();
            let name = path
                .file_name()
                .expect("path should point to a file")
                .to_str()
                .expect("path should contain ascii-only")
                .to_string();

            Self {
                name,
                library_path: path,
                plugin_dyn_id: OnceCell::new(),
                library_fns: OnceCell::new(),
            }
        }

        /// for debugging purposes
        pub fn name(&self) -> &str {
            &self.name
        }

        fn build_fn(&self) -> Result<BuildFn, libloading::Error> {
            let library_fns = self.library_fns.get_or_init(|| {
                let loaded_lib = unsafe { Library::new(&self.library_path) }.unwrap();

                const GLUE_SYM_NAME: &[u8; 31] = b"__hot_reload_plugin_build_glue\0";
                type GlueFn = unsafe extern "C" fn() -> *const PluginGlue;

                let glue_fn = *unsafe { loaded_lib.get::<GlueFn>(GLUE_SYM_NAME) }.unwrap();
                let glue_ptr: *const PluginGlue = unsafe { glue_fn() };
                let glue = unsafe { &*glue_ptr };

                const VERSION_WARNING: &str =
                    "Plugin wasn't compiled with the same toolchain version as plugin library";
                assert_eq!(glue.unit_type_id, ().type_id(), "{}", VERSION_WARNING);
                if let Err(old_plugin_dyn_id) = self.plugin_dyn_id.set(glue.plugin_dyn_id) {
                    assert_eq!(glue.plugin_dyn_id, old_plugin_dyn_id);
                };

                LibraryFns {
                    build_fn: glue.build.clone(),
                    _lib: loaded_lib,
                }
            });

            Ok(library_fns.build_fn)
        }
    }

    impl Plugin for DynPlugin {
        fn dyn_id(&self) -> DynId {
            *self.plugin_dyn_id.get().unwrap()
        }

        fn build(&self, app: &mut App) {
            let build_fn = self.build_fn().unwrap();
            unsafe { build_fn(app) }
        }
    }
}
