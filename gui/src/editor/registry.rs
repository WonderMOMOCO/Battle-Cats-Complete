use std::path::Path;

use kore::common::architecture;
use kore::domains::settings::EditorMode;

use crate::app::{theme, Page};
use crate::common::feedback::LOCKED_NOTICE;

use kore::systems::animation::authoring;

use crate::domains::studio;

use super::{classify, figures, ground, prose, Action, AnimTarget, CatTarget, Context, EnemyTarget, FileTarget, Format, GroundTarget, Item, LevelTarget, ProseTarget, Scope};

const BINARY_NOTICE: &str = "Cannot open binary format";
const NO_CHANNEL_NOTICE: &str = "This part has no channels in this animation";
const UNALIGNED_NOTICE: &str = "This model declares no unit divisors, and the revision that reads offsets reads those too";
const NO_OFFSET_NOTICE: &str = "This model declares no offset rows to remove";
const NOT_DRAWN_NOTICE: &str = "This part is not drawn on the current frame";
const EVERY_CHANNEL_NOTICE: &str = "This part already has every channel";
const NO_CLIP_NOTICE: &str = "Select an animation clip to add channels";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Verb {
    Edit,
    Replace,
    Open,
    Find,
    Add,
    Sync,
    Delete,
}

impl Verb {
    fn nested(self) -> &'static str {
        match self {
            Verb::Edit => "Edit in app",
            Verb::Replace => "Replace from disk",
            Verb::Open => "Open in program",
            Verb::Find => "Find in folder",
            Verb::Add => "Add to mod",
            Verb::Sync => "Sync from game",
            Verb::Delete => "Delete from disk",
        }
    }

    fn flat(self, name: &str, mount: &str) -> String {
        match self {
            Verb::Edit => format!("Edit \"{name}\" in \"{mount}\""),
            Verb::Replace => format!("Replace \"{name}\" in \"{mount}\""),
            Verb::Open => format!("Open \"{name}\" in \"{mount}\""),
            Verb::Find => format!("Find \"{name}\" in \"{mount}\""),
            Verb::Add => format!("Add \"{name}\" to \"{mount}\""),
            Verb::Sync => format!("Sync \"{name}\" with game in \"{mount}\""),
            Verb::Delete => format!("Delete \"{name}\" from \"{mount}\""),
        }
    }

    fn label(self, name: &str, mount: &Mount<'_>, labelling: Labelling) -> String {
        match labelling {
            Labelling::Verbose => self.flat(name, mount.name),
            Labelling::Terse => self.nested().to_owned(),
            Labelling::Named => name.to_owned(),
        }
    }
}

fn absent(mount: &Mount<'_>) -> String {
    format!("File does not exist in \"{}\"", mount.name)
}

fn manage(subject: &str, mount: &Mount<'_>) -> String {
    format!("Manage \"{subject}\" in \"{}\"", mount.name)
}

#[derive(Clone, Copy)]
struct Mount<'a> {
    name: &'a str,
    target: Option<&'a str>,
    locked: bool,
}

fn mount<'a>(active_mod: Option<&'a str>, unlocked: bool) -> Mount<'a> {
    match active_mod {
        Some(name) => Mount { name, target: Some(name), locked: false },
        None => Mount { name: architecture::GAME, target: None, locked: !unlocked },
    }
}

#[derive(Clone, Copy)]
enum Confirm {
    Never,
    Overwrite,
}

#[derive(Clone, Copy)]
enum Style {
    Flat,
    Nested,
}

impl Style {
    fn for_payloads(payloads: &[Payload<'_>]) -> Self {
        let live = payloads.iter().filter(|payload| !payload.scopes.is_empty()).count();

        if live > 1 { Style::Nested } else { Style::Flat }
    }
}

#[derive(Clone, Copy)]
enum Labelling {
    Verbose,
    Terse,
    Named,
}

impl Mount<'_> {
    fn item(&self, label: String, action: Action, confirm: Confirm) -> Item {
        if self.locked {
            return Item::disabled(label, LOCKED_NOTICE);
        }

        let item = Item::new(label, action);

        match confirm {
            Confirm::Overwrite => item.confirming(),
            Confirm::Never if self.target.is_none() => item.confirming(),
            Confirm::Never => item,
        }
    }
}

enum Primary<'a> {
    Replace,
    Animation(&'a AnimTarget),
    Cat(&'a CatTarget),
    Enemy(&'a EnemyTarget),
    Prose(&'a ProseTarget),
    Level(&'a LevelTarget),
    Ground(&'a GroundTarget),
    Unsupported(Verb),
}

struct Payload<'a> {
    key: &'a str,
    mount: Mount<'a>,
    scopes: Vec<Scope<'a>>,
    primary: Primary<'a>,
    values: EditorMode,
}

pub(super) fn items(context: &Context) -> Vec<Item> {
    let mut items = Vec::new();

    if !context.enabled {
        return items;
    }

    if context.page == Page::Files
        && let Some(file) = context.file.as_ref()
    {
        files(&mut items, file);

        return items;
    }

    if let Some(target) = context.offsets.as_ref() {
        return offsets(target);
    }

    if let Some(target) = context.channels.as_ref() {
        return channels(&target.channels);
    }

    let payloads = payloads(context);
    let style = Style::for_payloads(&payloads);

    for payload in &payloads {
        items.extend(payload.render(style));
    }

    items
}

fn offsets(target: &studio::Offsets) -> Vec<Item> {
    let label = format!("New offset at \"Row {}\"", target.rows);

    let adding = match target.addable {
        true => Item::new(label, Action::AddOffset),
        false => Item::disabled(label, UNALIGNED_NOTICE),
    };

    let dropping = match target.rows {
        0 => Item::disabled("No offsets".to_owned(), NO_OFFSET_NOTICE),
        1 => Item::new("Remove \"Row 0\"".to_owned(), Action::DropOffset { row: 0 }).confirming(),
        rows => Item::list(
            "Remove offset".to_owned(),
            (0..rows)
                .map(|row| {
                    Item::new(format!("Row {row}"), Action::DropOffset { row }).confirming_terse()
                })
                .collect(),
        ),
    };

    vec![adding, dropping]
}

fn channels(target: &studio::Channels) -> Vec<Item> {
    let part = target.part;
    let subject = &target.label;
    let held = &target.present;

    let absent: Vec<(i32, String)> = authoring::kinds()
        .into_iter()
        .filter(|kind| !held.iter().any(|(known, _)| known == kind))
        .map(|kind| (kind, authoring::kind_label(kind).to_owned()))
        .collect();

    let adding = match (target.channelled, absent.as_slice()) {
        (false, _) => Item::disabled(format!("Add channel to \"{subject}\""), NO_CLIP_NOTICE),
        (_, []) => Item::disabled(format!("Add channel to \"{subject}\""), EVERY_CHANNEL_NOTICE),
        (_, [(kind, label)]) => Item::new(
            format!("Add \"{label}\" to \"{subject}\""),
            Action::AddChannel { part, kind: *kind },
        ),
        _ => Item::list(
            format!("Add channel to \"{subject}\""),
            absent
                .into_iter()
                .map(|(kind, label)| Item::new(label, Action::AddChannel { part, kind }))
                .collect(),
        ),
    };

    let dropping = match held.as_slice() {
        [] => Item::disabled(format!("Remove channel from \"{subject}\""), NO_CHANNEL_NOTICE),
        [(kind, track)] => Item::new(
            format!("Remove \"{}\" from \"{subject}\"", authoring::kind_label(*kind)),
            Action::DropChannel { track: *track },
        )
        .confirming(),
        _ => Item::list(
            format!("Remove channel from \"{subject}\""),
            held.iter()
                .map(|(kind, track)| {
                    Item::new(authoring::kind_label(*kind).to_owned(), Action::DropChannel { track: *track })
                        .confirming_terse()
                })
                .collect(),
        ),
    };

    let finding = match target.locatable {
        true => Item::new(format!("Find \"{subject}\""), Action::Locate { part }),
        false => Item::disabled(format!("Find \"{subject}\""), NOT_DRAWN_NOTICE),
    };

    vec![
        finding,
        Item::new(format!("New part in \"{subject}\""), Action::AddPart { parent: Some(part) }),
        adding,
        dropping,
        Item::new(format!("Delete \"{subject}\""), Action::DropPart { part }).confirming(),
    ]
}

fn payloads(context: &Context) -> Vec<Payload<'_>> {
    let mut payloads = Vec::new();

    if let Some(target) = context.animation.as_ref() {
        let mount = mount(target.active_mod.as_deref(), target.unlocked);

        payloads.push(Payload {
            key: target.asset.key(),
            scopes: target.asset.scopes(mount.target.is_some()),
            mount,
            primary: Primary::Animation(target),
            values: context.values,
        });
    }

    if context.page == Page::Cats {
        for cat in &context.cats {
            let mount = mount(cat.active_mod.as_deref(), cat.unlocked);
            let present = present(&mount, cat.mod_copy.as_deref(), Some(&cat.source));

            payloads.push(Payload {
                key: &cat.file,
                scopes: vec![Scope { name: &cat.file, source: Some(&cat.source), present }],
                mount,
                primary: Primary::Cat(cat),
                values: context.values,
            });
        }
    }

    if context.page == Page::Enemies {
        for enemy in &context.enemies {
            let mount = mount(enemy.active_mod.as_deref(), enemy.unlocked);
            let present = present(&mount, enemy.mod_copy.as_deref(), Some(&enemy.source));

            payloads.push(Payload {
                key: &enemy.file,
                scopes: vec![Scope { name: &enemy.file, source: Some(&enemy.source), present }],
                mount,
                primary: Primary::Enemy(enemy),
                values: context.values,
            });
        }
    }

    if let Some(icon) = context.icon.as_ref() {
        let mount = mount(icon.active_mod.as_deref(), icon.unlocked);

        payloads.push(Payload {
            key: &icon.asset.internal,
            scopes: icon.asset.scopes(mount.target.is_some()),
            mount,
            primary: Primary::Replace,
            values: context.values,
        });
    }

    if let Some(banner) = context.banner.as_ref() {
        let mount = mount(banner.active_mod.as_deref(), banner.unlocked);

        payloads.push(Payload {
            key: &banner.key,
            scopes: banner.scopes(mount.target.is_some()),
            mount,
            primary: Primary::Replace,
            values: context.values,
        });
    }

    for target in &context.levels {
        let mount = mount(target.active_mod.as_deref(), target.unlocked);

        payloads.push(Payload {
            key: target.asset.key(),
            scopes: target.asset.scopes(mount.target.is_some()),
            mount,
            primary: Primary::Level(target),
            values: context.values,
        });
    }

    for target in &context.assets {
        let mount = mount(target.active_mod.as_deref(), target.unlocked);

        payloads.push(Payload {
            key: target.asset.key(),
            scopes: target.asset.scopes(mount.target.is_some()),
            mount,
            primary: Primary::Unsupported(Verb::Edit),
            values: context.values,
        });
    }

    if let Some(target) = context.ground.as_ref() {
        let mount = mount(target.active_mod.as_deref(), target.unlocked);
        let scopes = vec![Scope {
            name: target.file.as_str(),
            source: Some(target.game.as_path()),
            present: Some(target.game.as_path()),
        }];

        payloads.push(Payload {
            key: target.file.as_str(),
            scopes,
            mount,
            primary: Primary::Ground(target),
            values: context.values,
        });
    }

    for target in &context.prose {
        let mount = mount(target.active_mod.as_deref(), target.unlocked);

        payloads.push(Payload {
            key: target.asset.key(),
            scopes: target.asset.scopes(mount.target.is_some()),
            mount,
            primary: Primary::Prose(target),
            values: context.values,
        });
    }

    payloads
}

impl Payload<'_> {
    fn render(&self, style: Style) -> Vec<Item> {
        if self.scopes.is_empty() {
            return Vec::new();
        }

        let mut plans = match self.primary {
            Primary::Prose(target) => plans(target, &self.scopes, self.mount.target),
            _ => Vec::new(),
        };

        match style {
            Style::Flat => self.flat(&mut plans),
            Style::Nested => vec![self.nested(&mut plans)],
        }
    }

    fn flat(&self, plans: &mut [Option<prose::Plan>]) -> Vec<Item> {
        if let [only] = self.scopes.as_slice() {
            return self.bare(0, only, plans, Labelling::Verbose);
        }

        let mut columns: Vec<(Verb, Vec<Item>)> = Vec::new();

        for (at, scope) in self.scopes.iter().enumerate() {
            for (verb, item) in self.verbs(at, scope, plans, Labelling::Named) {
                match columns.iter_mut().find(|(known, _)| *known == verb) {
                    Some((_, listed)) => listed.push(item),
                    None => columns.push((verb, vec![item])),
                }
            }
        }

        let subject = self.subject();

        columns
            .into_iter()
            .filter_map(|(verb, mut listed)| {
                if listed.len() > 1 {
                    return Some(Item::list(verb.flat(&subject, self.mount.name), listed));
                }

                listed.pop().map(|only| {
                    let named = verb.flat(&only.label, self.mount.name);

                    only.relabel(named)
                })
            })
            .collect()
    }

    fn nested(&self, plans: &mut [Option<prose::Plan>]) -> Item {
        let children = match self.scopes.as_slice() {
            [only] => self.bare(0, only, plans, Labelling::Terse),
            scopes => {
                let mut per_file = Vec::with_capacity(scopes.len());

                for (at, scope) in scopes.iter().enumerate() {
                    per_file.push(Item::list(
                        scope.name.to_owned(),
                        self.bare(at, scope, plans, Labelling::Terse),
                    ));
                }

                per_file
            }
        };

        Item::list(manage(&self.subject(), &self.mount), children)
    }

    fn bare(
        &self,
        at: usize,
        scope: &Scope<'_>,
        plans: &mut [Option<prose::Plan>],
        labelling: Labelling,
    ) -> Vec<Item> {
        self.verbs(at, scope, plans, labelling).into_iter().map(|(_, item)| item).collect()
    }

    fn subject(&self) -> String {
        match self.scopes.as_slice() {
            [only] => only.name.to_owned(),
            _ => Path::new(self.key)
                .file_stem()
                .map_or_else(|| self.key.to_owned(), |stem| stem.to_string_lossy().into_owned()),
        }
    }

    fn verbs(
        &self,
        at: usize,
        scope: &Scope<'_>,
        plans: &mut [Option<prose::Plan>],
        labelling: Labelling,
    ) -> Vec<(Verb, Item)> {
        let mut items = Vec::new();
        let target_mod = self.mount.target.map(str::to_owned);

        match self.primary {
            Primary::Unsupported(verb) => {
                items.push((verb, Item::unsupported(verb.label(scope.name, &self.mount, labelling))));
            }
            Primary::Ground(target) => {
                let plan = ground::plan(
                    target.label.clone(),
                    scope.source.or(scope.present).unwrap_or(&target.game),
                    target_mod.clone(),
                    self.values,
                );

                items.push((
                    Verb::Edit,
                    self.mount.item(
                        Verb::Edit.label(scope.name, &self.mount, labelling),
                        Action::EditGround(plan),
                        Confirm::Never,
                    ),
                ));
            }
            Primary::Level(level) => {
                if let Some(plan) = level_plan(level, scope, target_mod, self.values) {
                    items.push((
                        Verb::Edit,
                        self.mount.item(
                            Verb::Edit.label(scope.name, &self.mount, labelling),
                            Action::EditFigures(plan),
                            Confirm::Never,
                        ),
                    ));
                }
            }
            Primary::Animation(target) => {
                let plan = super::anim_plan(target, self.mount.target.map(str::to_owned));
                let item =
                    Item::new(Verb::Edit.label(scope.name, &self.mount, labelling), Action::EditAnimation(plan));

                let item = match (self.mount.target, self.mount.locked) {
                    (None, false) => item.confirming(),
                    _ => item,
                };

                items.push((Verb::Edit, item));
            }
            Primary::Replace => items.push((Verb::Replace, replace(scope, &self.mount, labelling))),
            Primary::Cat(cat) => items.push((
                Verb::Edit,
                self.mount.item(
                    Verb::Edit.label(scope.name, &self.mount, labelling),
                    Action::EditFigures(cat_plan(cat, target_mod, self.values)),
                    Confirm::Never,
                ),
            )),
            Primary::Enemy(enemy) => items.push((
                Verb::Edit,
                self.mount.item(
                    Verb::Edit.label(scope.name, &self.mount, labelling),
                    Action::EditFigures(enemy_plan(enemy, target_mod, self.values)),
                    Confirm::Never,
                ),
            )),
            Primary::Prose(_) => {
                if let Some(plan) = plans.get_mut(at).and_then(Option::take) {
                    items.push((
                        Verb::Edit,
                        self.mount.item(
                            Verb::Edit.label(scope.name, &self.mount, labelling),
                            Action::EditProse(plan),
                            Confirm::Never,
                        ),
                    ));
                }
            }
        }

        items.push((Verb::Open, open(scope, &self.mount, labelling)));
        items.push((Verb::Find, find(scope, &self.mount, labelling)));
        items.extend(sync(scope, &self.mount, labelling).map(|item| (Verb::Sync, item)));
        items.push((Verb::Delete, delete(scope, &self.mount, labelling)));

        items
    }
}

fn open(scope: &Scope<'_>, mount: &Mount<'_>, labelling: Labelling) -> Item {
    let label = Verb::Open.label(scope.name, mount, labelling);

    let Some(path) = scope.present else {
        return Item::disabled(label, absent(mount));
    };

    match classify(path) {
        Format::Binary => Item::disabled(label, BINARY_NOTICE),
        Format::Text | Format::Image => Item::new(label, Action::Open { source: path.to_path_buf() }),
    }
}

fn find(scope: &Scope<'_>, mount: &Mount<'_>, labelling: Labelling) -> Item {
    let label = Verb::Find.label(scope.name, mount, labelling);

    let Some(path) = scope.present else {
        return Item::disabled(label, absent(mount));
    };

    Item::new(label, Action::Find { source: path.to_path_buf() })
}

fn replace(scope: &Scope<'_>, mount: &Mount<'_>, labelling: Labelling) -> Item {
    let label = Verb::Replace.label(scope.name, mount, labelling);

    if mount.locked {
        return Item::disabled(label, LOCKED_NOTICE);
    }

    let Some(target_mod) = mount.target else {
        let Some(path) = scope.present else {
            return Item::disabled(label, absent(mount));
        };

        let action = Action::Replace { file: scope.name.to_owned(), target_mod: None, game: Some(path.to_path_buf()) };

        return Item::new(label, action).confirming();
    };

    let action =
        Action::Replace { file: scope.name.to_owned(), target_mod: Some(target_mod.to_owned()), game: None };

    let item = Item::new(label, action);

    if scope.present.is_some() { item.confirming() } else { item }
}

fn sync(scope: &Scope<'_>, mount: &Mount<'_>, labelling: Labelling) -> Option<Item> {
    let active = mount.target?;
    let game = scope.source?;

    let label = Verb::Sync.label(scope.name, mount, labelling);

    let item = Item::new(
        label,
        Action::Sync {
            file: scope.name.to_owned(),
            target_mod: active.to_owned(),
            game: game.to_path_buf(),
        },
    );

    Some(if scope.present.is_some() { item.confirming() } else { item })
}

fn delete(scope: &Scope<'_>, mount: &Mount<'_>, labelling: Labelling) -> Item {
    let label = Verb::Delete.label(scope.name, mount, labelling);

    if mount.locked {
        return Item::disabled(label, LOCKED_NOTICE);
    }

    let Some(path) = scope.present else {
        return Item::disabled(label, absent(mount));
    };

    Item::new(label, Action::Delete { source: path.to_path_buf() }).confirming()
}

pub(super) fn prose_plans(target: &ProseTarget) -> Vec<prose::Plan> {
    let mount = mount(target.active_mod.as_deref(), target.unlocked);

    plans(target, &target.asset.scopes(mount.target.is_some()), mount.target).into_iter().flatten().collect()
}

fn plans(target: &ProseTarget, scopes: &[Scope<'_>], target_mod: Option<&str>) -> Vec<Option<prose::Plan>> {
    scopes
        .iter()
        .map(|scope| {
            let source = scope.source.or(scope.present)?;

            let made = prose::plan(
                target.subject,
                target.row,
                [target.label.as_str(), scope.name].join(theme::HEADER_SEPARATOR),
                scope.name.to_owned(),
                source,
                target_mod.map(str::to_owned),
            );

            Some(made.over(target.rows.clone()))
        })
        .collect()
}

fn present<'a>(mount: &Mount<'_>, mod_copy: Option<&'a Path>, game: Option<&'a Path>) -> Option<&'a Path> {
    if mount.target.is_some() { mod_copy } else { game }
}

pub(super) fn cat_plan(cat: &CatTarget, target_mod: Option<String>, values: EditorMode) -> figures::Plan {
    figures::plan(figures::Subject::Cat, figures::Address::Line(cat.form), cat.title(), &cat.source, target_mod, values)
}

pub(super) fn enemy_plan(enemy: &EnemyTarget, target_mod: Option<String>, values: EditorMode) -> figures::Plan {
    figures::plan(figures::Subject::Enemy, figures::Address::Line(enemy.row), enemy.title(), &enemy.source, target_mod, values)
}

pub(super) fn level_plan(
    level: &LevelTarget,
    scope: &Scope<'_>,
    target_mod: Option<String>,
    values: EditorMode,
) -> Option<figures::Plan> {
    let source = scope.source.or(scope.present)?;

    let made = figures::plan(level.subject, level.address, level.label.clone(), source, target_mod, values);

    Some(made.anchored(level.anchor))
}

fn files(items: &mut Vec<Item>, file: &FileTarget) {
    if file.folder {
        return;
    }

    let mount = mount(file.active_mod.as_deref(), file.unlocked);
    let browsing = Mount { name: &file.mount, target: None, locked: false };
    let scope = Scope { name: &file.name, source: file.game.as_deref(), present: Some(&file.source) };

    if file.mount == architecture::GAME
        && let Some(active) = mount.target
    {
        let confirm = if file.mod_copy.is_some() { Confirm::Overwrite } else { Confirm::Never };

        items.push(mount.item(
            Verb::Add.flat(&file.name, active),
            Action::Add { source: file.source.clone(), target_mod: active.to_owned() },
            confirm,
        ));
    }

    items.push(open(&scope, &browsing, Labelling::Verbose));
    items.push(find(&scope, &browsing, Labelling::Verbose));

    if file.mount != architecture::GAME {
        items.extend(sync(&scope, &mount, Labelling::Verbose));
        items.push(discard(file));

        return;
    }

    if mount.target.is_some() {
        return;
    }

    if mount.locked {
        items.push(Item::disabled(Verb::Delete.flat(&file.name, &file.mount), LOCKED_NOTICE));

        return;
    }

    items.push(discard(file));
}

fn discard(file: &FileTarget) -> Item {
    Item::new(Verb::Delete.flat(&file.name, &file.mount), Action::Delete { source: file.source.clone() })
        .confirming()
}
