# Testing 

Before testing ensure that you have installed the neccessary prerequisites listed in [building](building.md). The app has two primary components, the Svelte & Typescript frontend and the Rust backend. There are tests for both of these.

### Typescript tests

to run the typescript tests (which are written using vitest), you can simply run 

```bash
    npm test
```

to get a code coverage report, run 

```bash
    npm run coverage
```

This will also generate a coverage directory with the coverage report 

### Rust tests 

The rust tests are written using cargo's built in testing suite. To run, use the following steps 

```bash
    cd src-tauri
    cargo test 
```

To generate a coverage report, run the following command (also from within the src-tauri directory). Note that for this to work, you will need to already have llvm-cov installed on your system. You can follow the setup instructions [here](https://github.com/taiki-e/cargo-llvm-cov?tab=readme-ov-file#installation) to install llvm-cov.

```bash
    cargo coverage
```