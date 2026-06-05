/*
 * micro:bit V2 (nRF52833) 内存布局 - 配合 SoftDevice S113 v7.3.0
 *
 * Flash 总大小: 512KB (0x00000000 - 0x00080000)
 * RAM 总大小:   128KB (0x20000000 - 0x20020000)
 *
 * SoftDevice S113 v7.3.0 占用:
 *   Flash: 0x00000000 - 0x00026000 (152KB)
 *   RAM:   0x20000000 - 0x20002AD8 (约 11KB 取决于配置)
 *
 * 应用程序可用:
 *   Flash: 0x00026000 - 0x00080000 (360KB)
 *   RAM:   0x20002AD8 - 0x20020000 (约 117KB)
 */
MEMORY
{
  /* SoftDevice S113 v7.3.0 占用 Flash 前 0x26000 字节 */
  FLASH : ORIGIN = 0x00026000, LENGTH = 360K

  /* SoftDevice S113 v7.3.0 占用 RAM 前 0x2AD8 字节 */
  RAM   : ORIGIN = 0x20002AD8, LENGTH = 0x1D528
}
