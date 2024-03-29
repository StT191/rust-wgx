
use std::{
    collections::{HashMap, HashSet}, ops::Range, borrow::Cow, sync::Arc,
    path::{Path, PathBuf}, fs:: read_to_string,
};
use lazy_static::lazy_static;
use regex_lite::Regex;


// error and result types
pub type Error = String;
pub type Res<T> = Result<T, Error>;


#[derive(Debug)]
struct Include { path: Arc<Path>, source_range: Range<usize> }


// module
#[derive(Debug)]
pub struct Module {
    includes: Vec<Include>,
    dependencies: HashSet<Arc<Path>>,
    source: Box<str>,
    code: Box<str>,
}


// import regexes
lazy_static! {
    static ref TEST_REGEXES: [Regex; 2] = [
        Regex::new(r#"(\n|}|;|^)(\s*)(?://)?\s*&\s*include\s+(?:"|')(.+?)(?:"|')\s*(;|\n)"#).unwrap(),
        Regex::new(r#"(\n|}|;|^)(\s*)/\*\s*&\s*include\s+(?:"|')(.+?)(?:"|')\s*;?\s*\*/()"#).unwrap(),
    ];
}

impl Module {

    fn load_source(source: Cow<str>) -> Self {

        let mut includes = Vec::new();

        for test_regex in TEST_REGEXES.iter() {

            let mut from = 0;

            while let Some(captures) = test_regex.captures_at(&source, from) {

                let path: Arc<Path> = AsRef::<Path>::as_ref(&captures[3]).into();

                let matched = captures.get(0).unwrap();

                let prefix = &captures[1];

                let start =
                    matched.start() + prefix.len() +
                    if prefix == "}" || prefix == ";" { captures[2].len() } else { 0 }
                ;

                let end = matched.end() - if &captures[4] == "\n" { 1 } else { 0 };

                includes.push(Include { path, source_range: start..end });

                from = matched.end() - captures[4].len();
            }
        }

        Self {
            includes, dependencies: HashSet::new(),
            source: source.into(), code: "".into(),
        }
    }


    fn load_source_from_path(path: impl AsRef<Path>) -> Res<Self> {

        let path = path.as_ref();

        // fetch source
        let source = read_to_string(path).map_err(|err| format!("{err} '{}'", path.display()))?;

        Ok(Self::load_source(source.into()))
    }
}



// helper
fn parent_path(path: &Path) -> Res<&Path> {
    path.parent().ok_or_else(|| format!("invalid path '{}'", path.display()))
}

fn normpath(path: &Path) -> PathBuf {

    let mut normal = PathBuf::new();
    let mut level: usize = 0;

    for part in path.iter() {
        if part == ".." {
            if level != 0 { normal.pop(); level -= 1 }
            else { normal.push(".."); }
        }
        else if part != "." {
            normal.push(part);
            level += 1;
        }
    }

    normal
}


// modules

pub struct ModuleCache { map: HashMap<Arc<Path>, Module> }

impl ModuleCache {

    fn resolve_module(&mut self, module_trace: &mut Vec<Arc<Path>>, path: Arc<Path>) -> Res<&mut Module> {

        if module_trace.contains(&path) { return Err(format!(
            "circular dependency {} from {}",
            path.display(),
            module_trace.last().unwrap().display(),
        )) }

        if !self.map.contains_key(&path) {
            let mut module = Module::load_source_from_path(&path)?;

            let dir_path = parent_path(&path)?;

            module_trace.push(path.clone());
            module.resolve_includes(self, module_trace, &Arc::from(dir_path))?;
            module_trace.pop();

            self.map.insert(path.clone(), module);
        }

        Ok(self.map.get_mut(&path).unwrap())
    }
}


impl Module {

    fn resolve_includes(&mut self, cache: &mut ModuleCache, module_trace: &mut Vec<Arc<Path>>, dir_path: &Path) -> Res<()> {

        let mut code = self.source.to_string();

        for include in self.includes.iter().rev() {

            let include_path = normpath(&dir_path.join(&include.path));
            let include_dir_path = parent_path(&include_path)?;

            let module = cache.resolve_module(module_trace, Arc::from(include_path.as_ref()))?;

            for dependency in &module.dependencies {
                let include_path = normpath(&include_dir_path.join(dependency));
                self.dependencies.insert(include_path.into());
            }

            self.dependencies.insert(include_path.into());

            code.replace_range(
                include.source_range.clone(),
                &module.code,
            );
        }

        self.code = code.into();

        Ok(())
    }


    // module loading

    pub fn load<'a>(path: impl AsRef<Path>, source_code: impl Into<Cow<'a ,str>>) -> Res<Module> {
        let path = Arc::from(normpath(path.as_ref()));
        let mut cache = ModuleCache::new();
        cache.load(&path, source_code)?;
        Ok(cache.map.remove(&path).unwrap())
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Res<Self> {
        let path = Arc::from(normpath(path.as_ref()));
        let mut cache = ModuleCache::new();
        cache.load_from_path(&path)?;
        Ok(cache.map.remove(&path).unwrap())
    }

    // accessors

    pub fn source(&self) -> &str { self.source.as_ref() }
    pub fn code(&self) -> &str { self.code.as_ref() }

    pub fn dependencies(&self) -> impl Iterator<Item=&Path> {
        self.dependencies.iter().map(|path| path.as_ref())
    }
}


// validation

use naga::{front::wgsl, valid::{ValidationFlags, Validator, Capabilities}};

fn validate(source: &str) -> Res<()> {
    wgsl::parse_str(source)
    .map_err(|err| err.emit_to_string_with_path(source, ""))
    .and_then(|module|
        match Validator::new(ValidationFlags::all(), Capabilities::all()).validate(&module) {
            Ok(_) => Ok(()),
            Err(err) => Err(err.emit_to_string_with_path(source, "")),
        }
    )
}



impl ModuleCache {

    pub fn new() -> Self { Self { map: HashMap::new() } }

    pub fn module(&self, path: impl AsRef<Path>) -> Option<&Module> {
        self.map.get(path.as_ref().into())
    }

    pub fn modules(&self) -> impl Iterator<Item=(&Path, &Module)> {
        self.map.iter().map(|(key, module)| (key.as_ref(), module))
    }


    fn load_helper(&mut self, path: &Path, source_code: Option<Cow<str>>) -> Res<&Module> {

        let path = Arc::from(normpath(path.as_ref()));
        let dir_path = parent_path(&path)?;

        let mut module = if let Some(source_code) = source_code {
            Module::load_source(source_code)
        } else {
            Module::load_source_from_path(&path)?
        };

        module.resolve_includes(self, &mut Vec::new(), &dir_path)?;

        validate(module.code())?;

        self.map.insert(Arc::from(path.as_ref()), module);

        Ok(self.map.get(&path).unwrap())
    }

    pub fn load<'a>(&mut self, path: impl AsRef<Path>, source_code: impl Into<Cow<'a ,str>>) -> Res<&Module> {
        self.load_helper(path.as_ref(), Some(source_code.into()))
    }

    pub fn load_from_path(&mut self, path: impl AsRef<Path>) -> Res<&Module> {
        self.load_helper(path.as_ref(), None)
    }
}