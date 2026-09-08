# Mods
You can create, view, and inject mods into APKs using the `Mods` page.

## Initialize
To initialize a mod, there are two paths: `Automatic` or `Manual`.

### Automatic
Under the `Mods` page there is a button on the top left labeled "Add Mod." Click on this button, it will open a popup with the following options:
- **New**: Creates a new mod folder for you using the name you provide.
- **Android**: Import an existing mod from an installed APK on an Android device. Requires Keys & IV.
- **BCM**: Import files from a `.zip` or `.bcm` archive.
- **Pack**: Import from the games proprietary `.pack` and `.list` format. Requires Keys & IV.

All mods initialized through this avenue will have a simple internal structure containing the following folders: `patch`, `icons`, and `loose`.

### Manual
Enter the `mods` folder next to the Battle Cats Complete binary (create it if it doesn't exist) and create a new folder inside of it. The folder name will be the name of your Mod in the Mods list.

## Develop
There are multiple different avenues that you can take to develop your Mod in different ways. Detailed below is information on how modified assets are handled by BCC as well as avenues BCC can help you modify.

### Assets
Within a mods directory, all files act as an **override** that replaces the vanilla asset, and only that one asset. This means the **only** files you need inside this directory are your **custom** modified assets, not the whole database. This behavior mimics how the game behaves when custom assets are injected.

All files edited under a mod are automatically added to that mod for you. Replacing existing files will always ask you for confirmation before destroying the original.

### Editor
If you are looking to modify data such as Cat & Enemy Abilities, Cat & Enemy Images, as well as Stage Data, read the `Editor` page available on the left sidebar.

### Studio
If you are looking to modify data such as Entity Spritesheets, Models, as well as Animations, read the `Studio` page available on the left sidebar

## Export
You can export a mod under the `Mods` page by selecting your mod and clicking "Export Mod," in which you are given 3 options:
- **APK**: Export directly into a `.apk` file, or optionally, if you have the APKEditor add-on, `.xapk` files as well. Requires Keys & IV.
- **BCM**: Export into a `.bcm` archive, which is just a renamed zip file made to signify "this is a battle cats mod."
- **Pack**: Export into the games proprietary `.pack` and `.list` format for pack-specific injection. Requires Keys & IV.