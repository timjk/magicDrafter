# Magic Drafter
Toy application that displays the community rankings of a given card when drafting in `MagicArena`.

## Description
Created primarily to learn Rust as such expect a lot of churn in the codebase as I learn best-practices, new approaches, new libraries etc...

## Features
- Creates a local sqlite db with card rankings for `Core Set 2020`

## Setup
In order for this tool to work you need to enable detailed logging in MTG Arena:
1. Start MTG Arena
2. Click the "Options" gear (top right)
3. Click "View Account" (bottom)
4. Check "Detailed Logs (Plugin Support)"
5. Restart MTG Arena

## Future features
- CLI that displays card rankings for the given pack
- Store scryfall & community rankings in a google doc, update local db from there
- Display a UI in vulkan overtop of `MagicArena`

## Rust concepts to learn
- Testing
- Project layout
- Handle multiple errors types in a single function
- Investigate `async/await`

## License
[MIT](./LICENSE-MIT)