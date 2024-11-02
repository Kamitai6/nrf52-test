target extended-remote :3333

monitor arm semihosting enable

file target/thumbv8m.main-none-eabihf/release/stm32h562

load

continue