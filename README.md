# sm64js-mmo-server

### Links

[Main Website: sm64js.com](https://sm64js.com)

[Discord Server](https://discord.gg/7UaDnJt)

## What is this?

This is the server code for sm64js-mmo

## Prerequisites

- Postgres Database, e.g. via Docker.
- `libpq-dev` for Debian based distros.
- copy and rename the file `.env.template` to `.env` and insert your environment variables.

## Development

The server with also serve client assets, so they first need to be compiled.
Assuming that you cloned this repository from the [Monorepo](https://github.com/sm64js/sm64js-mmo),
you will have to navigate to the `client` folder and run `yarn build:rust` once after installing dependencies.
For development you can instead run `yarn webpack --mode development --env rust` or use webpack-dev-server.
