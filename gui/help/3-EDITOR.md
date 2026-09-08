# Editor
You can invoke the Editor using the **Context Menu**, allowing you to modify and create modded files for a currently enabled Mod. Mods are created through `Mods > Add Mod`, and are enabled through `Mods > YourModHere > Enable Mod`, where `YourModHere` is the name of the mod you'd like to enable.

**Disclaimer:** The Editor is currently an opt-in **Nightly** feature enabled through `Settings > General > Behavior > Enable Nightly Features`. This means it is potentially unstable and may be buggy. The Editor handles the creation, deletion, and writing of files. It is recommended you back-up your Mod's files before using the Editor while it is in this state.

## Context Menu
To select files you want to work with, you must right-click to open the Context Menu. You can select a specific file to handle, and select an action to take with it. There are two different options driving Context Menu file access under `Settings > Files > Context Scope`: `Broad` and `Specific`.

### Broad
Gives the Context Menu access to all currently loaded files related to the page you are seeing on your screen. Makes access easy and quick, but can feel bloated or confusing due to an abundant amount of options the Context Menu provides upfront. The default option.

### Specific
Gives the Context Menu access to files related only to what you right-clicked on. Allows you to decide what to edit by scanning the UI instead of scanning a verbose Context Menu list for your specific file. Requires basic knowledge of what files are used where in BCC for an efficient workflow.

## Editing Mode
There are two editing modes: `Raw` and `Resolved`. The mode can be changed under `Settings > Files > Editor > Editor Mode`.

### Raw
The real raw file values, with no fancy UI, options, or normalization. It's essentially a text editor that attaches a name to every column and/or row.

### Resolved
Abstracts away from raw file values to provide an easy-to-use but slightly more restrictive editing experience. It displays flags as buttons, clamps input to prevent unreasonable or impossible values, and transforms magic numbers into human-readable words and selections. The default option.

## Buffer Character
When editing a clamped field, you may sometimes notice that BCC resolves the value mid-write, making it difficult to write the value you want as it transforms inputs such as `100` into `200` when you start writing the value at `1`.

Numeric input fields accept a buffer character: `!`. When this character is included at the start of a field, your value will not be resolved or written until the field is "unfocused." Fields become "unfocused" when you click anywhere outside the input field.gui