# hkx-extractor

This repository provides a way that converts from hkx animation to human readable format and imports into blender.

# Requirements

- Windows 11

- Havok Content Tools 2010

- Python 3.11

  - numpy

  - quaternionic

- Blender 4.0

# Usage

1. Move animation hkx files to `resources/hkx64files`.

2. Run `main.py` for converting from animation hkx to csv.

3. Open `blender/main.blend` by blender and run create_armature.py and load_animation.py in blender's script window.

# Credits

- hkxPoser uses Havok(R). (C) Copyright 1999-2008 Havok.com Inc. (and its Licensors). All Rights Reserved. See www.havok.com for details.

- https://github.com/opparco/hkxPoser

- https://github.com/opparco/hkdump

# License

MIT License
