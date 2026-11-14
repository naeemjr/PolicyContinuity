//! The oracle decides what a request denotes from its own model of the
//! fixture. This checks that model against the kernel: for every
//! development scenario, in every fixture state its schedule can reach,
//! the object the model says the request denotes must be the object the
//! kernel resolves the request to.

use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

use policycontinuity::fixture::{Fixture, Graph, Kind};
use policycontinuity::model::{Region, Split};
use policycontinuity::oracle::{denote, lexically_contained, Denotation};
use policycontinuity::record::RequestPath;
use policycontinuity::scenario::{generate, Scenario};

fn arena(name: &str) -> tempfile::TempDir {
    let parent = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("model-soundness");
    std::fs::create_dir_all(&parent).unwrap();
    let parent = std::fs::canonicalize(parent).unwrap();
    tempfile::Builder::new()
        .prefix(name)
        .tempdir_in(parent)
        .unwrap()
}

fn kernel_object(fixture: &Fixture, scenario: &Scenario) -> usize {
    let path = match &scenario.request {
        RequestPath::Relative(text) => fixture.root_path().join(text),
        RequestPath::AbsoluteInWorkspace(loc) => fixture.path(loc),
    };
    let meta = std::fs::metadata(&path)
        .unwrap_or_else(|e| panic!("{}: kernel could not resolve the request: {e}", scenario.id));
    let matches = fixture.objects_with_identity((meta.dev(), meta.ino()));
    assert_eq!(matches.len(), 1, "{}: ambiguous identity", scenario.id);
    matches[0]
}

fn model_object(graph: &Graph, scenario: &Scenario) -> usize {
    match denote(graph, &scenario.request) {
        Denotation::Object(id) => id,
        other => panic!("{}: the model says {other:?}", scenario.id),
    }
}

#[test]
fn the_model_agrees_with_the_kernel_in_every_reachable_state() {
    let tmp = arena("states-");
    let mut checked = 0;
    let mut outside = 0;
    for seed in 1..=10 {
        for (i, scenario) in generate(seed, Split::Development).iter().enumerate() {
            let base = tmp.path().join(format!("s{seed}-{i}"));
            let mut fixture = Fixture::create(base, seed).unwrap();
            for step in &scenario.setup {
                fixture.apply(step).unwrap();
            }
            let mut compare = |fixture: &Fixture| {
                let model = model_object(fixture.graph(), scenario);
                assert_eq!(model, kernel_object(fixture, scenario), "{}", scenario.id);
                assert_eq!(fixture.graph().node(model).kind, Kind::File);
                checked += 1;
                outside += usize::from(fixture.graph().node(model).region == Region::Outside);
            };
            compare(&fixture);
            for mutation in &scenario.schedule {
                for step in &mutation.steps {
                    fixture.apply(step).unwrap();
                }
                compare(&fixture);
            }
            assert!(fixture.registry_stable(), "{}", scenario.id);
            fixture.destroy().unwrap();
        }
    }
    assert!(checked > 1000, "only {checked} states were compared");
    assert!(outside > 100, "only {outside} states denoted a sentinel");
}

/// The two policies must disagree somewhere, or they would be one. In this
/// matrix they disagree in one direction only.
#[test]
fn lexical_containment_accepts_requests_that_reach_a_sentinel() {
    let tmp = arena("policies-");
    let (mut lexical_only, mut object_only) = (0, 0);
    for (i, scenario) in generate(1, Split::Development).iter().enumerate() {
        let mut fixture = Fixture::create(tmp.path().join(format!("p{i}")), 1).unwrap();
        for step in &scenario.setup {
            fixture.apply(step).unwrap();
        }
        let object = model_object(fixture.graph(), scenario);
        let authorized = fixture.graph().node(object).region == Region::Authorized;
        let lexical = lexically_contained(&scenario.request);
        lexical_only += usize::from(lexical && !authorized);
        object_only += usize::from(authorized && !lexical);
        fixture.destroy().unwrap();
    }
    // A link to a sentinel: fine as text, wrong as an object.
    assert!(lexical_only > 0);
    // No request in the matrix is contained as an object but not as text:
    // a request that leaves and re-enters the root normalises back under
    // it, and requests with `..` are only generated on link-free fixtures.
    assert_eq!(object_only, 0);
}
