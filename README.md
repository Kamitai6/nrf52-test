# setup
1. install rustup
2. ```rustup target add thumbv7em-none-eabihf```
3. ```curl -O https://raw.githubusercontent.com/microsoft/uf2/master/utils/uf2conv.py```
4. ```curl -O https://raw.githubusercontent.com/microsoft/uf2/master/utils/uf2families.json```
5. ```cargo install cargo-binutils```
6. ```rustup component add llvm-tools-preview```

# build and flash
7. ```cargo flash```

フラッシュ時は、リセットボタンを2回連続で押してディスク認識させる
フラッシュ後は、1度リセットしないとUSBCDCが認識されない