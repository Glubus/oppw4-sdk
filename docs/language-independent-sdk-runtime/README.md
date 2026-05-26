# Language-Independent SDK Runtime

Ce dossier sert de tableau de bord pour decouper le chantier "SDK runtime
independant du langage" en missions que plusieurs IA peuvent prendre sans se
marcher dessus.

## But final

Construire un runtime SDK ou les hooks du jeu produisent des evenements types
Rust, ou les frontends Lua/JS/Rhai ne sont que des adaptateurs, et ou les mods
de haut niveau peuvent ecrire des regles propres pour rewards, ranks,
difficulty, player et futures features sans dependre directement de Lua ni de
noms reverse comme `param4`, `reward_out` ou `global+0x1d756`.

## Regles d'or

- Le loader reste petit. Rien de rank, rewards, difficulty ou data dans le
  loader.
- `sdk.runtime` possede les hooks, contexts runtime, events et mutations.
- Lua est un frontend, pas le coeur de l'architecture.
- Les probes reverse restent read-only par defaut.
- Toute hypothese installee dans le jeu doit etre testable et launchable.
- Ne pas toucher `D:\SteamLibrary\steamapps\common\OPPW4\dinput8.dll`.
- Installer le SDK avec le package sans loader, puis copier les plugins.
- Les nouveaux fichiers de code doivent suivre les patterns existants du repo.

## Docs a lire avant de prendre une tache

- `docs/LANGUAGE-INDEPENDENT-SDK-RUNTIME.md`
- `docs/PLUGIN-API.md`
- `docs/HANDOFF-2026-05-25-runtime-rank-difficulty-rewards.md`
- `docs/ROADMAP.md`
- `docs/reverse-notes/rank-pipeline-ghidra-2026-05-25.md`
- `docs/reverse-notes/difficulty-reward-ghidra-2026-05-20.md`

## Decoupage global

### Step 1 - Cadrage et types core

Objectif: lancer en parallele les deux taches qui ne dependent de rien: cadrage
ownership et types core.

Fichiers:

- `step-1/1-1.md`: inventaire et limites d'ownership.
- `step-1/1-2.md`: types core rewards/rank/difficulty/player.

Ces deux taches peuvent etre faites par deux IA en meme temps.

### Step 2 - Event bus runtime

Objectif: construire le bus event/mutation apres les types core.

Fichier:

- `step-2/2-1.md`: bus d'evenements runtime.

### Step 3 - Rewards MVP et observabilite

Objectif: brancher le premier flux concret sur le core: rewards commit, Lua
`on_commit`, et logs/signals. `3-1` doit etre merge avant de finaliser `3-2` et
`3-3`, mais les IA peuvent preparer leur partie en parallele.

Fichiers:

- `step-3/3-1.md`: pipeline mutations rewards MVP.
- `step-3/3-2.md`: adaptateur Lua rewards `on_commit`.
- `step-3/3-3.md`: signaux/logs/debug pour validation.

### Step 4 - Suppression staged rules

Objectif: retirer les anciennes APIs rewards staged et garder uniquement le
pipeline events/mutations.

Fichier:

- `step-4/4-1.md`: ancienne note de migration, remplacee par la politique
  sans compat legacy.

### Step 5 - Packaging et test jeu

Objectif: verifier que le socle est buildable, installable et testable dans le
jeu sans remplacer le loader.

Fichier:

- `step-5/5-1.md`: packaging et test jeu launchable.

## Graphe de dependances

```text
step-1/1-1  \
             -> step-2/2-1 -> step-3/3-1 -> step-4/4-1 -> step-5/5-1
step-1/1-2  /                    \-> step-3/3-2
                                  \-> step-3/3-3
```

### Step 2 - Rewards director propre

Objectif: brancher berry en vrai, puis preparer medals/items, crew points et
souls une fois les champs confirmes.

Sortie attendue:

- `RewardCommitEvent` stable.
- `RewardMutation::MultiplyBerry` appliquee dans le hook.
- API Lua: `sdk.rewards.on_commit(function(ctx) ... end)`.
- Les souls restent stub tant que le reverse n'est pas confirme.

### Step 3 - Rank director propre

Objectif: separer les seuils helper, le cap Easy et la politique de merge.

Sortie attendue:

- API publique pour `rank.set_easy_s_rankable(true)`.
- API seuils count/time seulement sur sources confirmees.
- API merge/global rank plus tard, apres validation de `FUN_1412dd790`.

### Step 4 - Difficulty director virtuel

Objectif: faire un "Nightmare effect" sans ajouter de cinquieme difficulte menu.

Sortie attendue:

- Regles sur combat pressure et tables spawn/proba confirmees.
- Hooks ou patches limites, explicites, configurables.
- Aucun patch aveugle de HP/attack/defense sans reverse confirme.

### Step 5 - Data dumper

Objectif: transformer les logs runtime en data editable dans `oppw4-data`.

Sortie attendue:

- Dumps mission/reward/rank/difficulty par mission.
- Donnees source regenerables.
- Notes d'evidence rangees par mission.

### Step 6 - Frontends multiples

Objectif: sortir Lua du coeur et rendre JS/TS possible.

Sortie attendue:

- `sdk_lua` frontend optionnel.
- API TypeScript cible documentee.
- Runtime core testable sans VM.

## Definition of done globale

- `cargo test` passe.
- Le package SDK se build.
- Le jeu demarre avec les plugins installes.
- La feature activee a une hypothese claire a tester.
- Les logs disent quoi verifier, pas seulement "loaded".
- Les docs expliquent ce qui est confirme et ce qui reste reverse.
