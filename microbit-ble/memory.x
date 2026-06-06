/*
 * micro:bit V2 (nRF52833) 内存布局 - 配合 SoftDevice S113 v7.3.0
 *
 * Flash 总大小: 512KB (0x00000000 - 0x00080000)
 * RAM 总大小:   128KB (0x20000000 - 0x20020000)
 *
 * SoftDevice S113 v7.3.0 占用:
 *   Flash: 0x00000000 - 0x0001C000 (112KB, 从 SD info struct 读取)
 *   RAM:   0x20000000 - 0x20003400 (约 13KB，取决于 BLE 配置)
 *
 * 应用程序可用:
 *   Flash: 0x0001C000 - 0x00080000 (400KB)
 *   RAM:   0x20003400 - 0x20020000 (约 115KB)
 *
 * 注意: RAM 起始地址需要根据 SoftDevice 实际需求调整。
 *       如果程序 panic 提示 "too little RAM for softdevice"，
 *       需要将 ORIGIN 增大到 panic 信息中给出的地址。
 *       如果提示 "giving more RAM than needed"，可以适当减小。
 */
MEMORY
{
  /* SoftDevice S113 v7.3.0 占用 Flash 前 0x1C000 字节 (112KB) */
  FLASH : ORIGIN = 0x0001C000, LENGTH = 400K

  /* SoftDevice S113 v7.3.0 占用 RAM 前 0x3400 字节 */
  RAM   : ORIGIN = 0x20003400, LENGTH = 0x1CC00
}
