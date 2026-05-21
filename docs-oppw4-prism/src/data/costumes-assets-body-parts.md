# Costumes, Assets, Body Parts

Costumes are the main way to organize model and texture assets.

The intended shape is:

```text
character
  costumes[]
    id
    label
    assets[]
    body_parts[]
```

Assets can describe models, textures, portraits, voices, and related files.
Body parts are data-defined because characters can have multiple weapons or
special parts. Prefer explicit names such as `body`, `weapon_01`, `weapon_02`,
`left_arm`, or `right_arm` when the source data supports them.

Do not use one generic `weapon` field when the character can carry multiple
weapons.
