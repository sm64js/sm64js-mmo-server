# sm64js-mmo-server

## Links

[Main Website: sm64js.com](https://sm64js.com)

[Discord Server](https://discord.gg/7UaDnJt)

## Prerequisites

- Postgres database, e.g. via [Docker](https://hub.docker.com/_/postgres/).
- `libpq-dev` for Debian based distros.
- copy and rename the file `.env.template` to `.env` and insert your environment variables.

## Development

The server will also serve client assets, so they first need to be compiled.
You will have to navigate to the `client` folder and run `yarn build:rust` once after installing dependencies.
For development you can instead run `yarn webpack --mode development --env rust`
or use webpack-dev-server (WIP).

In your `.env` file, you don't have to insert all variables for local development.
It is only mandatory to have a running Postgres database, thus you need to set the `DATABASE_URL` variable.
Currently only Google sign-in is mocked, so you will have to use this,
if you cannot set up the Discord environment variables.
