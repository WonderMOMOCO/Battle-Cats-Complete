use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use kore::systems::animation::authoring::{Imgcut, Maanim, Mamodel};

use super::{Field, Gizmo};

const DEPTH: usize = 25;
const COALESCE: Duration = Duration::from_millis(700);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Tag {
    Key(usize, Field),
    Loop,
    Ease(usize),
    Keys,
    Field(usize),
    Axis(usize),
    Parts,
    Cut(usize, usize),
    Cuts,
    Gizmo(usize, Gizmo),
    Bulk,
}

impl Tag {
    pub(super) fn subject(self) -> Subject {
        match self {
            Tag::Key(..) | Tag::Loop | Tag::Ease(_) | Tag::Keys => Subject::Anim,
            Tag::Field(_) | Tag::Axis(_) | Tag::Parts => Subject::Model,
            Tag::Cut(..) | Tag::Cuts => Subject::Cuts,
            Tag::Gizmo(_, Gizmo::Channel) => Subject::Anim,
            Tag::Gizmo(_, Gizmo::Model) => Subject::Model,
            Tag::Bulk => Subject::Rig,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Subject {
    Anim,
    Model,
    Cuts,
    Rig,
}

pub(super) enum Shot {
    Anim(PathBuf, Maanim),
    Model(PathBuf, Mamodel),
    Cuts(PathBuf, Imgcut),
    Rig(Vec<(PathBuf, Vec<u8>)>),
}

impl Shot {
    fn anchor(&self) -> Option<&Path> {
        match self {
            Shot::Anim(path, _) | Shot::Model(path, _) | Shot::Cuts(path, _) => Some(path),
            Shot::Rig(_) => None,
        }
    }
}

struct Entry {
    tag: Tag,
    shot: Shot,
    at: Instant,
}

#[derive(Default)]
pub(super) struct History {
    entries: Vec<Entry>,
}

impl History {
    pub(super) fn wanted(&self, tag: Tag, anchor: Option<&Path>) -> bool {
        let Some(last) = self.entries.last() else {
            return true;
        };

        last.tag != tag
            || last.shot.anchor().map(Path::to_path_buf) != anchor.map(Path::to_path_buf)
            || last.at.elapsed() >= COALESCE
    }

    pub(super) fn push(&mut self, tag: Tag, shot: Shot) {
        self.entries.push(Entry { tag, shot, at: Instant::now() });

        while self.entries.len() > DEPTH {
            self.entries.remove(0);
        }
    }

    pub(super) fn pop(&mut self) -> Option<Shot> {
        self.entries.pop().map(|entry| entry.shot)
    }

    pub(super) fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cuts() -> Imgcut {
        Imgcut::parse(b"[imgcut]\n1\nx.png\n1\n0,0,4,4,box\n").expect("the sample parses")
    }

    #[test]
    fn typing_into_one_cell_costs_a_single_entry() {
        // Every keystroke asks; only the first within the window is taken.
        let mut history = History::default();
        let path = PathBuf::from("studio/a/x.imgcut");
        let tag = Tag::Cut(0, 0);

        assert!(history.wanted(tag, Some(&path)));
        history.push(tag, Shot::Cuts(path.clone(), cuts()));

        assert!(!history.wanted(tag, Some(&path)));
        assert!(history.wanted(Tag::Cut(0, 1), Some(&path)), "a different cell is a new edit");
        assert!(history.wanted(tag, Some(Path::new("studio/a/y.imgcut"))), "so is another file");
    }

    #[test]
    fn the_stack_never_grows_past_its_depth() {
        let mut history = History::default();
        let path = PathBuf::from("studio/a/x.imgcut");

        for at in 0..DEPTH + 10 {
            history.push(Tag::Cut(at, 0), Shot::Cuts(path.clone(), cuts()));
        }

        assert_eq!(history.entries.len(), DEPTH);
        assert_eq!(history.entries[0].tag, Tag::Cut(10, 0), "the oldest entries fall off the front");
    }
}

pub(super) fn shelve<T>(store: &mut Vec<(String, T)>, key: String, held: T, cap: usize) {
    store.retain(|(name, _)| *name != key);
    store.push((key, held));

    while store.len() > cap {
        store.remove(0);
    }
}

#[cfg(test)]
mod shelf_tests {
    use super::*;

    fn names(store: &[(String, u32)]) -> Vec<&str> {
        store.iter().map(|(name, _)| name.as_str()).collect()
    }

    #[test]
    fn the_shelf_holds_three_sets_and_evicts_the_oldest() {
        let mut store: Vec<(String, u32)> = Vec::new();

        for (at, name) in ["a", "b", "c", "d"].into_iter().enumerate() {
            shelve(&mut store, name.to_owned(), at as u32, 3);
        }

        assert_eq!(names(&store), vec!["b", "c", "d"], "a fell off the back");
    }

    #[test]
    fn re_shelving_a_set_moves_it_to_the_front_without_duplicating() {
        let mut store: Vec<(String, u32)> = Vec::new();

        for (at, name) in ["a", "b", "c"].into_iter().enumerate() {
            shelve(&mut store, name.to_owned(), at as u32, 3);
        }

        shelve(&mut store, "a".to_owned(), 99, 3);

        assert_eq!(names(&store), vec!["b", "c", "a"]);
        assert_eq!(store.last().map(|(_, held)| *held), Some(99), "and it carries the new value");

        shelve(&mut store, "d".to_owned(), 4, 3);

        assert_eq!(names(&store), vec!["c", "a", "d"], "b is now the oldest and goes");
    }
}
