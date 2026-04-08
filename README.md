# sparko-esp-std
ESP32 Implementation of Sparko Embedded Std
## Introduction
The idea of Sparko Embedded is to provide a platform for embedded applications for hardware such as ESP32 SoC boards. Sparko Embedded Std is a version of this platform which includes the standard Rust library which means that the heap and standard collections like ```Vec``` are all available for use. This crate contains code for that platform running on ESP32 based boards.

Example applications for various boards are available at sparko-embedded-examples.

## Features
This crate uses multiple features to support various different ESP32 boards. There are functional features like ```mono-led``` and ```rgb-led``` which are referenced in the code and which get activated by board level features like ```board-cyd``` and ```board-xaio-esp32c6```. Client crates should normally select exactly one board feature and no others.


## Development
In order to avoid compiler errors during development one of the board features should be enabled in VSCode settings (file ```.vscode/settings.json``` in the workspace root) and when building on the command line release mode and one board feature should be selected e.g. ```cargo build --release --features board-cyd```

## Supported Boards
The following boards are currently supported:

### board-cyd
The so called "Cheap Yellow Display" or more properly the **ESP32-2432S028R** is a 
development board has become known in the maker community as the “Cheap Yellow Display” or CYD for short. This development board, whose main chip is an ESP32-WROOM-32 module, comes with

- 2.8-inch TFT touchscreen LCD
- microSD card interface
- RGB LED
- built-in LDR (light-dependent resistor)
- all the required circuitry to program and apply power to the board.

Useful board information can be found at [Random Nerd Tutorials](https://randomnerdtutorials.com/?s=CYD)
I have read that there are clones of this board which have slight differences, the one I developed on cam from [Ali Express](https://www.aliexpress.com/item/1005008229897039.html)

### board-xiao-esp32c6
This is the [Seed Studio XIAO ESP32-C6](https://www.seeedstudio.com/Seeed-Studio-XIAO-ESP32C6-p-5884.html) board which combines 2.4GHz Wi-Fi 6 (802.11ax), Bluetooth 5(LE), and IEEE 802.15.4 radio connectivity with a C6 processor.
