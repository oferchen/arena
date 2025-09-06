# Contributing

## Formatting

- Run `npm run format` to format JavaScript, JSON, Markdown and other supported files.
- Run `cargo fmt --all` to format Rust code.

## Linting

- Run `npm run lint` to check formatting with Prettier.
- Run `cargo clippy --all-targets --all-features` to lint Rust code.

## Git hooks

Husky is configured to run `npm run lint` on pre-commit. Hooks are installed automatically after `npm install`. Remove the `.husky` directory if you prefer not to use Git hooks.
