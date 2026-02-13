# UB Add Shorthand

Automatically adds the current directory as a service to a Procfile when running `ub add` with no arguments.

## Usage

```bash
ub add
```

When run in a directory like `/path/to/myproject`, this automatically:
1. Extracts the directory name (e.g., `myproject`)
2. Adds an entry to the Procfile in the current directory: `myproject: <command>`
3. Creates a Procfile if one doesn't exist

## The Problem

Setting up a new service in ubermind requires manually editing the Procfile and choosing an appropriate name. For simple projects where the directory name is sufficient, this is repetitive.

## The Solution

`ub add` without arguments uses the current working directory's basename as the service name and prompts for (or infers) the command to run.

## Example

```bash
cd ~/projects/api-service
ub add
# Prompts for command or uses default based on project type
# Adds to Procfile: api-service: bun run dev
```

## Implementation

The command:
1. Gets the current working directory basename using `pwd` or equivalent
2. Determines the appropriate command (prompts user or detects from project files)
3. Appends to `./Procfile` or creates it if missing
4. Validates the Procfile format

## Edge Cases

- If a Procfile already has an entry with the same name, prompts for confirmation to overwrite
- If no Procfile exists, creates one with proper formatting
- If the directory name contains invalid characters for a service name, sanitizes it (lowercase, replace spaces/special chars with hyphens)
