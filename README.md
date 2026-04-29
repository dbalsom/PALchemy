
![palchemy_logo](doc/img/palchemy_logo_big.png)

# PALchemy

PALchemy is a set of Rust crates that are intended to assist with the dumping of PAL chips, but can also be used for various other chip manipulation and investigation tasks.

> [!WARNING]  
> This project is still under heavy initial development.
> Contents of this README may be somewhat "aspirational" and contain references to unimplemented or at least not-yet-pushed features.

## Rationale

Over the past few months, I've been reverse-engineering the [schematics for the Seequa Chameleon](https://github.com/dbalsom/seequa_chameleon), a quirky XT compatible-ish luggable machine from 1983. Its motherboard has 16 different PAL chips on it, about half of them of the registered type.  I didn't know much about PAL chips or how they are dumped before this, but I have been learning quite a bit in the process.  I've encountered various tools for working with PAL chips, but as with any somewhat obscure, technical field, the whole process of PAL dumping - and especially doing anything with the dump you have made - seemed rather cryptic.

PALchemy is both an attempt to learn how PALs work, and also to try to contribute an improvement to the tooling available for working with PAL chips. Time will tell if I succeed at doing the latter...

## What is a PAL, anyway?

A PAL chip is a simple programmable logic device or PLD. 'PAL' itself started out as a brand name, a product of a company called Monolithic Memories, Inc.  It stands for Programmable Array Logic, not to be confused with a Programmable Logic Array.  I know, right?

The name has stuck, which can lead to some confusion. A PAL is somewhat less configurable than a fully programmable logic device. It is comprised of a programmable OR array, with a fixed AND array.

Basically, it was a cost-effective way for board designers to potentially replace several discrete 74-series logic chips with a single DIP-20 or DIP-24 package - with the added bonus of being able to reprogram new PAL chips as needed instead of reworking a circuit board.

# Inspiration

One day, someone on my Discord server showed off an awesome hack where they were driving an ISA CGA card - without a computer, just using a T48 USB programmer to bit-bang an ISA slot adapter. That caught my attention.

This gave the me the idea to use the T48 for reading PALs - a convenient option that could possibly let you dump a PAL without a custom PCB or breadboard setup - just drop your PAL into the reader and go. In reality things are a bit more complicated, but in general this is possible for simple combinatorial PALs.

# Design

PALchemy's current incarnation is a set of Rust crate intended to provide a somewhat modular design.

 - **palcore**: Core logic functionality, data types, chip definition parsing
 - **palapp**: Tauri application framework
 - **palgui**: Leptos GUI logic, web components and styling
 - **palhal**: Hardware abstraction trait `GpioProvider`, and bundled implementations thereof
    - **teefo-rs**: A simple USB driver for the T-48 that provides `GpioProvider` trait. Based on [CABBIC](https://github.com/donohoe00/cabbic).
 - **palcli**: A plain CLI interface using `palcore` and `palhal` for non-interactive operations like combinatorial PAL dumping.

# Crates used

PALchemy is my first app using the Rust GUI framework [Tauri](https://v2.tauri.app/).  It is similar in concept to Electron, but uses the native system WebView, so it is a lot more lightweight. 

To avoid writing any Javascript, I'm also using [Leptos](https://leptos.dev/) for the first time, which is a framework for writing Web Components in Rust (that compile to Wasm).

Yes, this may seem like overkill for a PAL dumping application, but I had a specific UI vision in mind that I didn't think I could reasonably implement in my usual Rust gui toolkit, [egui](https://github.com/emilk/egui).

# License

Two crates in this repository are GPL3 only, **palhal** and **teefo-rs**, due to a respectable amount of snooping around in [minipro](https://gitlab.com/DavidGriffith/minipro) and the [DuPAL](https://github.com/DuPAL-PAL-DUmper/) projects.  The rest of the crates are dual licensed as either MIT or GPL3.

# Building

TODO...