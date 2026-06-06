/*
 * micro:bit V2 (nRF52833) memory layout - for use with SoftDevice S113 v7.3.0
 *
 * Flash total size: 512KB (0x00000000 - 0x00080000)
 * RAM total size:   128KB (0x20000000 - 0x20020000)
 *
 * SoftDevice S113 v7.3.0 usage:
 *   Flash: 0x00000000 - 0x0001C000 (112KB, read from SD info struct)
 *   RAM:   0x20000000 - 0x20003400 (approx. 13KB, depends on BLE config)
 *
 * Application available:
 *   Flash: 0x0001C000 - 0x00080000 (400KB)
 *   RAM:   0x20003400 - 0x20020000 (approx. 115KB)
 *
 * Note: RAM origin address may need adjustment based on SoftDevice requirements.
 *       If the program panics with "too little RAM for softdevice",
 *       increase ORIGIN to the address shown in the panic message.
 *       If it says "giving more RAM than needed", you can decrease it.
 */
MEMORY
{
  /* SoftDevice S113 v7.3.0 occupies first 0x1C000 bytes (112KB) of Flash */
  FLASH : ORIGIN = 0x0001C000, LENGTH = 400K

  /* SoftDevice S113 v7.3.0 occupies first 0x3400 bytes of RAM */
  RAM   : ORIGIN = 0x20003400, LENGTH = 0x1CC00
}
