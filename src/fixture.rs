//! Fixtures: the declared object graph and its on-disk materialisation.
//!
//! ```text
//! <base>/                 temporary workspace (object 0)
//!   root/                 the approved root   (object 1, authorized)
//!   <anything else>       outside the root, still inside the workspace
//! ```
//!
//! Every fixture is built from declarative [`Step`]s. Each step is applied
//! twice, in lock-step: to a pure in-memory [`Graph`] and to the disk. The
//! graph assigns every object a small integer id in creation order and
//! records which side of the policy boundary it was created on; the disk
//! side records the object's (device, inode) at creation. Together they are
//! the evaluator's hidden state. Nothing in this module is visible to an
//! adapter.

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::model::Region;
use crate::sys::{self, FileId};

/// Tracked fixture object id, assigned in creation order.
pub type ObjId = usize;

/// Name of the approved root directory inside every workspace.
pub const ROOT_NAME: &str = "root";

/// Placeholder used wherever a report needs to show the workspace path.
pub const WORKSPACE_TOKEN: &str = "<workspace>";

/// A location named by plain components relative to the workspace base.
/// A location never passes through a symlink.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loc(pub Vec<String>);

impl Loc {
    /// `Loc::parse("root/a/b")`.
    pub fn parse(text: &str) -> Self {
        Loc(text.split('/').map(str::to_string).collect())
    }

    pub fn join(&self, name: &str) -> Self {
        let mut parts = self.0.clone();
        parts.push(name.to_string());
        Loc(parts)
    }

    /// The same location with a suffix appended to its final name.
    pub fn with_suffix(&self, suffix: &str) -> Self {
        let mut parts = self.0.clone();
        if let Some(last) = parts.last_mut() {
            last.push_str(suffix);
        }
        Loc(parts)
    }

    pub fn region(&self) -> Region {
        if self.0.first().map(String::as_str) == Some(ROOT_NAME) {
            Region::Authorized
        } else {
            Region::Outside
        }
    }

    pub fn display(&self) -> String {
        self.0.join("/")
    }
}

/// What a symlink contains.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkText {
    /// Literal relative text.
    Relative(String),
    /// An absolute path to a workspace location, built at runtime.
    AbsoluteInWorkspace(Loc),
}

impl LinkText {
    pub fn display(&self) -> String {
        match self {
            LinkText::Relative(text) => text.clone(),
            LinkText::AbsoluteInWorkspace(loc) => format!("{WORKSPACE_TOKEN}/{}", loc.display()),
        }
    }
}

/// One declarative fixture step. Steps never delete anything: an object
/// that must disappear from a name is renamed aside. That keeps inode
/// numbers from being reused inside a run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// Create one directory. Its parent must exist.
    Dir(Loc),
    /// Create a regular file holding that object's canary.
    File(Loc),
    Symlink {
        at: Loc,
        target: LinkText,
    },
    /// Rename within one region. The destination must not exist.
    Rename {
        from: Loc,
        to: Loc,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Dir(BTreeMap<String, ObjId>),
    File,
    Symlink(LinkText),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub kind: Kind,
    pub region: Region,
    /// The directory currently holding this object; `None` for the base.
    pub parent: Option<ObjId>,
    pub name: String,
}

/// The declared object graph: a pure model of the fixture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Graph {
    nodes: Vec<Node>,
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}

impl Graph {
    /// The workspace base directory.
    pub const BASE: ObjId = 0;
    /// The approved root directory.
    pub const ROOT: ObjId = 1;

    /// A workspace holding only the empty approved root.
    pub fn new() -> Self {
        let base = Node {
            kind: Kind::Dir(BTreeMap::from([(ROOT_NAME.to_string(), Self::ROOT)])),
            region: Region::Outside,
            parent: None,
            name: String::new(),
        };
        let root = Node {
            kind: Kind::Dir(BTreeMap::new()),
            region: Region::Authorized,
            parent: Some(Self::BASE),
            name: ROOT_NAME.to_string(),
        };
        Self {
            nodes: vec![base, root],
        }
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn node(&self, id: ObjId) -> &Node {
        &self.nodes[id]
    }

    pub fn ids(&self) -> std::ops::Range<ObjId> {
        0..self.nodes.len()
    }

    /// The entry called `name` in directory `dir`, if `dir` is a directory
    /// and has one.
    pub fn child(&self, dir: ObjId, name: &str) -> Option<ObjId> {
        match &self.nodes[dir].kind {
            Kind::Dir(entries) => entries.get(name).copied(),
            _ => None,
        }
    }

    /// The object at `loc`, walking directory entries only.
    pub fn lookup(&self, loc: &Loc) -> Option<ObjId> {
        loc.0
            .iter()
            .try_fold(Self::BASE, |dir, name| self.child(dir, name))
    }

    /// Where an object currently sits, relative to the base.
    pub fn loc_of(&self, id: ObjId) -> Loc {
        let mut parts = Vec::new();
        let mut at = id;
        while let Some(parent) = self.nodes[at].parent {
            parts.push(self.nodes[at].name.clone());
            at = parent;
        }
        parts.reverse();
        Loc(parts)
    }

    fn parent_of(&self, loc: &Loc) -> Result<(ObjId, String), String> {
        let (name, dirs) = loc
            .0
            .split_last()
            .ok_or_else(|| "empty location".to_string())?;
        let dir = self
            .lookup(&Loc(dirs.to_vec()))
            .ok_or_else(|| format!("parent of `{}` does not exist", loc.display()))?;
        if !matches!(self.nodes[dir].kind, Kind::Dir(_)) {
            return Err(format!("parent of `{}` is not a directory", loc.display()));
        }
        Ok((dir, name.clone()))
    }

    fn entries_mut(&mut self, dir: ObjId) -> &mut BTreeMap<String, ObjId> {
        match &mut self.nodes[dir].kind {
            Kind::Dir(entries) => entries,
            _ => unreachable!("callers check the node is a directory"),
        }
    }

    fn create(&mut self, loc: &Loc, kind: Kind) -> Result<ObjId, String> {
        let (dir, name) = self.parent_of(loc)?;
        if self.child(dir, &name).is_some() {
            return Err(format!("`{}` already exists", loc.display()));
        }
        let id = self.nodes.len();
        self.nodes.push(Node {
            kind,
            region: loc.region(),
            parent: Some(dir),
            name: name.clone(),
        });
        self.entries_mut(dir).insert(name, id);
        Ok(id)
    }

    /// Apply one step to the model. Returns the id of the object the step
    /// created, if it created one.
    pub fn apply(&mut self, step: &Step) -> Result<Option<ObjId>, String> {
        match step {
            Step::Dir(loc) => self.create(loc, Kind::Dir(BTreeMap::new())).map(Some),
            Step::File(loc) => self.create(loc, Kind::File).map(Some),
            Step::Symlink { at, target } => {
                self.create(at, Kind::Symlink(target.clone())).map(Some)
            }
            Step::Rename { from, to } => {
                if from.region() != to.region() {
                    return Err(format!(
                        "rename `{}` -> `{}` crosses the policy boundary",
                        from.display(),
                        to.display()
                    ));
                }
                let id = self
                    .lookup(from)
                    .ok_or_else(|| format!("`{}` does not exist", from.display()))?;
                let (from_dir, from_name) = self.parent_of(from)?;
                let (to_dir, to_name) = self.parent_of(to)?;
                if self.child(to_dir, &to_name).is_some() {
                    return Err(format!("`{}` already exists", to.display()));
                }
                self.entries_mut(from_dir).remove(&from_name);
                self.entries_mut(to_dir).insert(to_name.clone(), id);
                self.nodes[id].parent = Some(to_dir);
                self.nodes[id].name = to_name;
                Ok(None)
            }
        }
    }
}

/// A fixture on disk, with its model and identity registry.
#[derive(Debug)]
pub struct Fixture {
    base: PathBuf,
    graph: Graph,
    /// (device, inode) of each object, recorded when it was created.
    identities: Vec<FileId>,
    nonce: u64,
}

impl Fixture {
    /// Create a fresh workspace at `base`, which must not exist yet and
    /// whose parent must be a fully resolved path.
    pub fn create(base: PathBuf, nonce: u64) -> io::Result<Self> {
        fs::create_dir(&base)?;
        let root = base.join(ROOT_NAME);
        fs::create_dir(&root)?;
        let identities = vec![sys::path_id(&base)?, sys::path_id(&root)?];
        Ok(Self {
            base,
            graph: Graph::new(),
            identities,
            nonce,
        })
    }

    pub fn base(&self) -> &Path {
        &self.base
    }

    pub fn root_path(&self) -> PathBuf {
        self.base.join(ROOT_NAME)
    }

    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    pub fn path(&self, loc: &Loc) -> PathBuf {
        loc.0
            .iter()
            .fold(self.base.clone(), |path, name| path.join(name))
    }

    /// The contents a tracked file is created with. It names the object, so
    /// what is read through a handle is a second witness to which object the
    /// handle is.
    pub fn canary(&self, id: ObjId) -> Vec<u8> {
        format!(
            "PC2|obj={id}|region={}|nonce={:016x}\n",
            self.graph.node(id).region,
            self.nonce
        )
        .into_bytes()
    }

    /// The marker a write operation puts at the start of its target.
    pub fn write_marker(&self) -> Vec<u8> {
        format!("PC2|write|nonce={:016x}|", self.nonce).into_bytes()
    }

    /// Apply one step to the model and to the disk.
    pub fn apply(&mut self, step: &Step) -> Result<(), String> {
        let created = self.graph.apply(step)?;
        let disk = match step {
            Step::Dir(loc) => fs::create_dir(self.path(loc)),
            Step::File(loc) => {
                let id = created.expect("a file step creates an object");
                // `create_new` never writes through an existing name.
                OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(self.path(loc))
                    .and_then(|mut file| file.write_all(&self.canary(id)))
            }
            Step::Symlink { at, target } => {
                let text = match target {
                    LinkText::Relative(text) => PathBuf::from(text),
                    LinkText::AbsoluteInWorkspace(loc) => self.path(loc),
                };
                std::os::unix::fs::symlink(text, self.path(at))
            }
            Step::Rename { from, to } => {
                let to_path = self.path(to);
                if fs::symlink_metadata(&to_path).is_ok() {
                    return Err(format!("`{}` already exists on disk", to.display()));
                }
                fs::rename(self.path(from), to_path)
            }
        };
        disk.map_err(|err| format!("applying {step:?}: {err}"))?;
        if let Some(id) = created {
            let loc = self.graph.loc_of(id);
            let identity = sys::path_id(&self.path(&loc))
                .map_err(|err| format!("recording identity of `{}`: {err}", loc.display()))?;
            debug_assert_eq!(self.identities.len(), id);
            self.identities.push(identity);
        }
        Ok(())
    }

    /// Every tracked object whose recorded identity equals `identity`.
    pub fn objects_with_identity(&self, identity: FileId) -> Vec<ObjId> {
        self.graph
            .ids()
            .filter(|id| self.identities[*id] == identity)
            .collect()
    }

    /// Whether every tracked object is still where the model says it is,
    /// still has the identity recorded at creation, and shares it with no
    /// other tracked object.
    pub fn registry_stable(&self) -> bool {
        let mut seen = std::collections::BTreeSet::new();
        self.graph.ids().all(|id| {
            let path = self.path(&self.graph.loc_of(id));
            seen.insert(self.identities[id])
                && sys::path_id(&path).is_ok_and(|now| now == self.identities[id])
        })
    }

    fn tracked_files(&self) -> impl Iterator<Item = ObjId> + '_ {
        self.graph
            .ids()
            .filter(|id| matches!(self.graph.node(*id).kind, Kind::File))
    }

    fn read_tracked(&self, id: ObjId) -> Option<Vec<u8>> {
        fs::read(self.path(&self.graph.loc_of(id))).ok()
    }

    /// Tracked files whose contents now begin with the write marker.
    pub fn files_holding_marker(&self) -> Vec<ObjId> {
        let marker = self.write_marker();
        self.tracked_files()
            .filter(|id| {
                self.read_tracked(*id)
                    .is_some_and(|c| c.starts_with(&marker))
            })
            .collect()
    }

    /// Outside-region files whose contents differ from their canary, or
    /// that can no longer be read.
    pub fn sentinels_modified(&self) -> Vec<ObjId> {
        self.tracked_files()
            .filter(|id| self.graph.node(*id).region == Region::Outside)
            .filter(|id| self.read_tracked(*id) != Some(self.canary(*id)))
            .collect()
    }

    /// Delete the workspace. `remove_dir_all` does not follow symlinks.
    pub fn destroy(self) -> io::Result<()> {
        fs::remove_dir_all(&self.base)
    }
}

/// The object id a canary names, if `content` is an untouched canary.
pub fn canary_object(content: &[u8]) -> Option<ObjId> {
    let text = std::str::from_utf8(content).ok()?;
    let rest = text.strip_prefix("PC2|obj=")?;
    rest.split('|').next()?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::scratch;

    fn link(at: &str, target: &str) -> Step {
        Step::Symlink {
            at: Loc::parse(at),
            target: LinkText::Relative(target.to_string()),
        }
    }

    fn sample() -> (tempfile::TempDir, Fixture) {
        let tmp = scratch();
        let mut fx = Fixture::create(tmp.path().join("ws"), 0xABCD).unwrap();
        for step in [
            Step::Dir(Loc::parse("root/data")),
            Step::File(Loc::parse("root/data/a.txt")),
            Step::Dir(Loc::parse("outside")),
            Step::File(Loc::parse("outside/s.txt")),
        ] {
            fx.apply(&step).unwrap();
        }
        (tmp, fx)
    }

    #[test]
    fn objects_get_ids_in_creation_order_and_a_fixed_region() {
        let (_tmp, fx) = sample();
        let g = fx.graph();
        assert_eq!(g.len(), 6);
        let a = g.lookup(&Loc::parse("root/data/a.txt")).unwrap();
        let s = g.lookup(&Loc::parse("outside/s.txt")).unwrap();
        assert_eq!((a, s), (3, 5));
        assert_eq!(g.node(a).region, Region::Authorized);
        assert_eq!(g.node(s).region, Region::Outside);
        assert_eq!(g.node(Graph::ROOT).region, Region::Authorized);
    }

    #[test]
    fn canaries_name_their_object() {
        let (_tmp, fx) = sample();
        let on_disk = fs::read(fx.base().join("outside/s.txt")).unwrap();
        assert_eq!(on_disk, fx.canary(5));
        assert_eq!(canary_object(&on_disk), Some(5));
        assert_eq!(canary_object(b"something else"), None);
        assert_eq!(canary_object(&fx.write_marker()), None);
    }

    #[test]
    fn rename_keeps_identity_and_updates_the_model() {
        let (_tmp, mut fx) = sample();
        let a = fx.graph().lookup(&Loc::parse("root/data/a.txt")).unwrap();
        let before = sys::path_id(&fx.base().join("root/data/a.txt")).unwrap();
        fx.apply(&Step::Rename {
            from: Loc::parse("root/data/a.txt"),
            to: Loc::parse("root/data/b.txt"),
        })
        .unwrap();
        assert_eq!(fx.graph().loc_of(a).display(), "root/data/b.txt");
        assert_eq!(fx.objects_with_identity(before), vec![a]);
        assert!(fx.registry_stable());
    }

    #[test]
    fn steps_that_would_clobber_or_cross_the_boundary_are_rejected() {
        let (_tmp, mut fx) = sample();
        assert!(fx
            .apply(&Step::File(Loc::parse("root/data/a.txt")))
            .is_err());
        assert!(fx.apply(&Step::Dir(Loc::parse("root/missing/x"))).is_err());
        let cross = Step::Rename {
            from: Loc::parse("outside/s.txt"),
            to: Loc::parse("root/s.txt"),
        };
        assert!(fx.apply(&cross).unwrap_err().contains("policy boundary"));
        assert!(fx.registry_stable());
    }

    #[test]
    fn a_symlink_is_its_own_object_and_is_not_followed_by_lookup() {
        let (_tmp, mut fx) = sample();
        fx.apply(&link("root/leak", "../outside/s.txt")).unwrap();
        let g = fx.graph();
        let leak = g.lookup(&Loc::parse("root/leak")).unwrap();
        assert!(matches!(g.node(leak).kind, Kind::Symlink(_)));
        assert_eq!(g.node(leak).region, Region::Authorized);
        assert_eq!(g.lookup(&Loc::parse("root/leak/x")), None);
        assert!(fx.registry_stable());
    }

    #[test]
    fn the_registry_notices_an_untracked_change() {
        let (_tmp, fx) = sample();
        fs::rename(
            fx.base().join("root/data/a.txt"),
            fx.base().join("root/data/gone.txt"),
        )
        .unwrap();
        assert!(!fx.registry_stable());
    }

    #[test]
    fn marker_and_sentinel_checks_see_writes() {
        let (_tmp, fx) = sample();
        assert!(fx.files_holding_marker().is_empty());
        assert!(fx.sentinels_modified().is_empty());

        fs::write(fx.base().join("root/data/a.txt"), fx.write_marker()).unwrap();
        assert_eq!(fx.files_holding_marker(), vec![3]);
        assert!(fx.sentinels_modified().is_empty());

        fs::write(fx.base().join("outside/s.txt"), fx.write_marker()).unwrap();
        assert_eq!(fx.files_holding_marker(), vec![3, 5]);
        assert_eq!(fx.sentinels_modified(), vec![5]);
    }

    #[test]
    fn destroy_removes_everything() {
        let (tmp, fx) = sample();
        fx.destroy().unwrap();
        assert_eq!(fs::read_dir(tmp.path()).unwrap().count(), 0);
    }
}
