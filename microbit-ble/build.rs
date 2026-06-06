use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() {
  // 将 memory.x 放入链接器搜索路径
  let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());
  File::create(out.join("memory.x"))
    .unwrap()
    .write_all(include_bytes!("memory.x"))
    .unwrap();

  // 生成中断符号别名链接脚本
  // nrf-pac 0.3 (embassy-nrf 使用) 和 nrf52833-pac (nrf-softdevice 使用)
  // 对同一中断使用不同的符号名称，需要提供别名映射
  //
  // 关键：nrf_pac 的向量表使用 EGU2_SWI2 符号名，而 nrf-softdevice 定义的
  // 中断处理函数名为 SWI2_EGU2。必须将 EGU2_SWI2 映射到 SWI2_EGU2，
  // 否则 SoftDevice 的 BLE 事件通知中断将无法被正确处理。
  let interrupt_aliases = r#"
/* nrf-pac <-> nrf52833-pac 中断符号别名 */
/* nrf_pac 使用 EGU2_SWI2，nrf-softdevice 定义处理函数为 SWI2_EGU2 */
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

  // 当 memory.x 变化时重新运行
  println!("cargo:rerun-if-changed=memory.x");
  println!("cargo:rerun-if-changed=build.rs");
}
