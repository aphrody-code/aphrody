<!--
SPDX-License-Identifier: Apache-2.0
SPDX-FileCopyrightText: 2026 aphrody contributors
-->

# assets/

Static media for the aphrody repository.

## `aphrody-demo.cast`

A short [asciinema v2](https://docs.asciinema.org/manual/asciicast/v2/) recording
showing `aphrody --version`, `mrx scan --root .`, and a `jq` view of the resulting
`monorepo-map.json`. Total runtime: ~3 s. Embedded in the top-level `README.md`
as visual proof of the CLI.

### Play locally

```sh
asciinema play assets/aphrody-demo.cast
```

### Play in a browser

Use [`asciinema-player`](https://docs.asciinema.org/manual/player/) (vanilla JS,
no Node required):

```html
<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/asciinema-player/dist/bundle/asciinema-player.css" />
<div id="player"></div>
<script src="https://cdn.jsdelivr.net/npm/asciinema-player/dist/bundle/asciinema-player.min.js"></script>
<script>
  AsciinemaPlayer.create('aphrody-demo.cast', document.getElementById('player'));
</script>
```

### Regenerate

```sh
asciinema rec assets/aphrody-demo.cast --overwrite \
  --title "aphrody --version + mrx scan demo" \
  --cols 100 --rows 30
```

Then run the commands above, exit the recording with `Ctrl-D`, and commit.
