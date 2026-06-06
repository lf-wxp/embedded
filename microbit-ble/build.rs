use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() {
  // Add memory.x to the linker search path
  let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());
  File::create(out.join("memory.x"))
    .unwrap()
    .write_all(include_bytes!("memory.x"))
    .unwrap();

  // Generate interrupt symbol alias linker script
  // nrf-pac 0.3 (used by embassy-nrf) and nrf52833-pac (used by nrf-softdevice)
  // use different symbol names for the same interrupts, requiring alias mapping
  //
  // Key: nrf_pac's vector table uses EGU2_SWI2 symbol name, while nrf-softdevice's
  // interrupt handler is named SWI2_EGU2. Must map EGU2_SWI2 to SWI2_EGU2,
  // otherwise SoftDevice's BLE event notification interrupt won't be handled properly.
  let interrupt_aliases = r#"
/* nrf-pac <-> nrf52833-pac interrupt symbol aliases */
/* nrf_pac uses EGU2_SWI2, nrf-softdevice defines handler as SWI2_EGU2 */
PROVIDE(CLOCK_POWER = DefaultHandler);
PROVIDE(UARTE0 = DefaultHandler);
PROVIDE(TWISPI0 = DefaultHandler);
PROVIDE(TWISPI1 = DefaultHandler);
PROVIDE(AAR_CCM = DefaultHandler);
PROVIDE(EGU0_SWI0 = DefaultHandler);
PROVIDE(EGU1_SWI1 = DefaultHandler);
PROVIDE(EGU2_SWI2 = SWI2_EGU2);
PROVIDE(EGU3_SWI3 = DefaultHandler);
PROVIDE(EGU4_SWI4 = DefaultHandler);
PROVIDE(EGU5_SWI5 = DefaultHandler);
PROVIDE(SPI2 = DefaultHandler);
"#;
  File::create(out.join("interrupt_aliases.x"))
    .unwrap()
    .write_all(interrupt_aliases.as_bytes())
    .unwrap();

  println!("cargo:rustc-link-search={}", out.display());

  // Re-run when memory.x changes
  println!("cargo:rerun-if-changed=memory.x");
  println!("cargo:rerun-if-changed=build.rs");
}
