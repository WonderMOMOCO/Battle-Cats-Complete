# Mods
You can create, view, and inject mods into APKs using the `Mods` page.

## Initialize
To initialize a mod, there are two paths: `Automatic` or `Manual`.

### Automatic
Under the `Mods` page there is a button on the top left labeled "Add Mod." Click on this button, it will open a popup with the following options:
- **New**: Creates a new mod folder for you using the name you provide.
- **Android**: Import an existing mod from an installed APK on an Android device. 
- **BCM**: Import files from a `.zip` or `.bcm` archive.
- **Pack**: Import from the games proprietary `.pack` and `.list` format.

All mods initialized through this avenue will have a simple internal structure containing the following folders: `patch`, `icons`, and `loose`.

### Manual
Enter the `mods` folder next to the Battle Cats Complete binary (create it if it doesn't exist) and create a new folder inside of it. The folder name will be the name of your Mod in the Mods list.

## Develop
There are multiple different avenues that you can take to develop your Mod in different ways. Detailed below is information on how modified assets are handled by BCC as well as what modified assets BCC can help you create.

### Assets
Within a mods directory, all files act as an **override** that replaces the vanilla asset, and only that one asset. This means the **only** files you need inside this directory are your **custom** modified assets, not the whole database. This behavior mimics how the game behaves when custom assets are injected.