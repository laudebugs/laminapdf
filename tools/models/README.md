# Models

[`models.toml`](models.toml) lists the models Lamina PDF's extraction plug-ins may use ([ADR 0030](../../docs/decisions/0030-extraction-output-and-model-plugins.md) §6). Each entry pins a file by revision, size and SHA-256, and links the sources its code and weights licences were read from.

- `cargo xtask models check` (part of `cargo xtask check docs`) fails when a code or weights licence is outside the allowed permissive set, an entry is malformed or unpinned, or a weights file (`.onnx`, `.safetensors`, `.pt` and others) is anywhere in the repository.
- `cargo xtask models fetch <name>` downloads a model into `target/models/<name>/<sha256>/` and keeps it only when its hash and size match.

To add a model, read its code and weights licences at their sources, add an entry with both links and the date, and say in the PR why the rules fall short where it helps. Models whose weights restrict commercial use are refused: the file's comments list the ones already read and why.

[`heron_reference.py`](heron_reference.py) records the reference regions that `laminapdf-layout-onnx`'s ignored test compares against, with the packages in [`requirements.txt`](requirements.txt). It is run by hand, never by tests.
