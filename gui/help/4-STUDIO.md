# Studio
The `Studio` page edits an Entity's animation files directly: its sprite sheet (`.png`), its cut list (`.imgcut`), its model (`.mamodel`), and its animations (`.maanim`). Every change is written to disk as you make it.

**Disclaimer:** Studio writes files in place and has no save step, only a per-set 25-change-history undo. Back up a set before working on it.

## Sets
A **set** is the group of files that make up one entity. Sets live in the `Studio` folder, one folder per set. A set under `Studio` is edited in place.

Files from anywhere else are copied before the first edit, never written where they sit:
- **Mod Enabled:** the files are copied into that Mod, then edited there.
- **No Mod, `game` Unlocked:** the files are edited in place. Unlock through `Settings > Files > Editor`.
- **Anything Else:** the files are copied into `Studio` under the set's name.

Renaming a set renames its folder, and is only allowed while the set is in `Studio`.

### Manage
`Manage` handles the files in a set, split into `Set` and `Animation`.

`Set` holds the three files that define the entity.
- The name field renames the set, and is read-only for a set living in a Mod.
- **Import Set** copies an existing entity's files input into it.
- **New Set** creates an empty set to build from scratch.
- The folder list picks any set already in `Studio`.
- **PNG**, **IMGCUT**, and **MAMODEL** replace a single file. A loaded file shows green with its name.
- **Open Folder** reveals the set on disk, and is only available for sets in `Studio`.

`Animation` holds the set's `.maanim` files.
- The name field renames the selected animation.
- **Add MAANIM** brings existing animation files into the set.
- **New MAANIM** creates an empty animation and loads it.
- **Remove MAANIM** drops the selected animation, and deletes the file for a set in `Studio`.

## Atlas
`Atlas` edits the sprite sheet's cut list. `Add Cut` appends a new region.
- **Set:** Allows you to right-click and drag to set the bounds of that cut.
- **Trim:** Resize the cut to remove any dead space.
- **Find:** Centers your camera on the part.
- **Select:** Selects the part, making its row entry blue and outline bold.

You can also Select parts by clicking on them in the Atlas viewer.

## Export
Unless a mod is enabled, `Export` zips the loaded set into the `exports` folder.

When a Mod is enabled, export will instead ask for you to give the zip a name, or to input the name or ID for a Cat or Enemy. If given a Cat or Enemy, the exporter will copy the target Entities file names as well as sheet ID and be added to the enabled Mod under the target Entities' identity.

## Entity
`Entity` edits the model and the selected animation together. The left tree lists the model's parts; expanding a part lists the **channels** animating it. Selecting either fills the table on the right.

A **channel** is one animated property of one part, such as its angle or opacity, holding the keyframes that drive it. A part may carry two channels of the same kind, in which case the earlier one is marked `overridden`, as only the last one takes effect.

Right-clicking a part offers to add a part beneath it, add or remove a channel, or delete it. Right-clicking the field table offers to add or remove an offset row.

`View` jumps the playhead to a keyframe. `Bound` loops playback across the segment that keyframe begins.

## Offset
The combo above `Export` picks the offset row the viewer places the entity by. The game always places an entity by one of these rows, so `None` shows a position the game never draws.

Studio and Utilities list the rows by number, as the file states them. Cats and Enemies name them instead: `Combat` and `Gacha` for a cat, `Combat` and `Base HP` for an enemy, with `Raw` for no row at all. A row the page does not name still appears by number.

The row is pinned to the part its first column names, usually the root. That part cannot be moved: the offset moves with it and the game draws it unmoved. Rotating and scaling it still work.

## Option
The `Option` column sits beside the table in `Entity` mode. Each row is a name and a value; clicking the name cycles the value, which can also be picked from its list. Rows are split across pages, stepped through by the arrows at the bottom of the column. `Gizmo` is covered under Gizmo, `Module` under Timeline, `Fault` under Warnings, and `Rig`, `Hierarchy`, `Selected`, `World` and `Origin` under Colors.

### Onionskin
Draws the frames around the current one as faded ghosts. Setting it to `Enabled` opens its settings, and closing those sets it back to `Disabled`.

`Before` and `After` are the two directions, each carrying:
- **Duration:** how many frames a ghost lasts before it fades away. Leave it empty to turn that direction off.
- **Color:** the tint applied to that direction's ghosts, as hex.

`Ghost` applies to both directions:
- **Delay:** how many frames apart ghosts are laid down.
- **Opacity:** how transparent the ghost is.

A direction draws as many ghosts as its `Delay` fits inside its `Duration`, so a `Delay` longer than the `Duration` leaves gaps where no ghost is alive.

### Entity
Picks which parts the viewer draws at all, rather than which overlays it draws over them.
- **Rig:** every part. The default option.
- **Hierarchy:** the selected part and its children.
- **Selected:** the selected part alone.
- **None:** nothing.

## Timeline
The table on the bottom has two readouts, switched by `Module` in the `Option` column. `Table` lists the selected part's values; `Timeline` draws its channels as lanes.

Each lane is one channel of the selected part, named on a card at its left and split into blocks between its keyframes. An `overridden` channel is drawn faded. Blocks past the end of a looping channel are its repeats, and cannot be grabbed.
- **Left Drag** moves the view. Sideways pans through frames, up and down scrolls through lanes; there is no scrollbar.
- **Scrolling** zooms around the cursor.
- **Left Click** to the right of the name cards moves the playhead.
- **Right Click** on a lane selects that channel and fills the keyframe table.
- **Right Drag** on the ends of or edge between two blocks moves that keyframe, and is the only edit the Timeline offers. A keyframe is held between its neighbors and cannot cross them, and reaches only as far as the frames on screen. Zoom out first to extend a channel further.

A green line marks frame 0, and a lime line marks the frame a loop restarts on, which sits under the green one whenever the two agree. The playhead is red while it matches the frame being played, and amber once the part's own channels have folded it, either resting on its last keyframe or wrapped back inside a loop. When amber the line sits where the part resolves while the frame count carries on past it, and an animation the file never ends stays amber from its first fold.

## Colors
The viewer draws its overlays in a fixed set of colors. Which ones appear is toggled in the `Option` column.
- **Red** outlines parts. Every part is outlined faintly; the selected one is outlined boldly.
- **Cyan** marks origins. Each part gets a dot at the point it rotates and scales around, and the selected part is joined to its parent's origin by a line.
- **Yellow** is the selected part's own direction, running from its origin out through the top of the part.
- **Purple** is the Gizmo. Enables live entity edits. It brightens while you work with it.
- **Green** is the world rather than any part: the ground line, the entity's height, and a mark at the world's origin.

## Gizmo
Parts can be posed directly in the viewer. Left click selects a part and reveals it in the tree; left click it again to deselect. Right-click pauses the viewer and allows you to right-click drag on certain areas of the selected part to pose it:
- **Middle** moves the part.
- **Edges & Corners** stretch it. Dragging an edge past the opposite one flips it.
- **The Ring** on the part's axis rotates it.
- **Scrolling** while holding changes its opacity.

The `Option` column picks what the Gizmo writes. This is disabled and forced to `Model` when no animation is loaded.

A root part, one with no parent, ignores its model X and Y; the game reads those columns only for a part that has a parent. The two fields are greyed in the table and the Gizmo refuses the drag on `Model`, pointing you at `Channel` instead. A part the offset row is pinned to cannot be moved on either setting, as covered under Offset.

### Channel
Writes to the selected animation, keyframing the current frame only. If the part has no channel for the property, one is created. This is the default option.

### Model
Writes to the model's rest pose, which affects every frame of every animation.

## Warnings
Studio marks the parts and channels the game would crash on, as row backgrounds in the `Entity` tree.
- **Red** is the part or channel that causes the crash.
- **Yellow** is a part holding a marked descendant, so a mark is visible without expanding the tree.

The animation buttons are marked too. An animation carrying a fault draws red, and the `Model` button draws red for a fault the rig holds on its own rather than one an animation brings.

Selecting a marked row explains it. Eight faults are detected: a zero scale divisor, a zero opacity divisor, two keyframes of one curved run sharing a frame, a part drawing from a sprite sheet the entity never loads, an offset row the game reads but the model does not declare, an offset row naming a part the model does not hold, an attack animation measuring no frames, and an attack animation that never ends and never leaves frame 0.

### Fault
`Fault` in the `Option` column picks the side the rig is checked against, as the game keeps one placement per side and faults on different things for each.
- **None:** no checking, and no marks.
- **Attack:** reads every animation as though it were the attack slot, which is the only way to reach the two attack faults on a set that is not installed. A fault found this way says so, as it only matters if that animation really is the unit's attack.
- **Both:** everything either side would fault on. The default option.
- **Cat:** forms and souls.
- **Enemy:** enemies.

Opening a cat or an enemy from a Mod or an unlocked `game` sets this for you, as the files say which side they belong to. Turn that off through `Settings > Studio > Auto Set Fault`.

## Undo
`Ctrl+Z` reverts the last change, up to 25 back. History is kept for the last three sets you loaded; loading a fourth drops the oldest.
