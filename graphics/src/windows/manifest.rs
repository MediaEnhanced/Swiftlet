//Media Enhanced Swiftlet Graphics Windows Manifest File
//MIT License
//Copyright (c) 2024 Jared Loewenthal
//
//Permission is hereby granted, free of charge, to any person obtaining a copy
//of this software and associated documentation files (the "Software"), to deal
//in the Software without restriction, including without limitation the rights
//to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
//copies of the Software, and to permit persons to whom the Software is
//furnished to do so, subject to the following conditions:
//
//The above copyright notice and this permission notice shall be included in all
//copies or substantial portions of the Software.
//
//THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
//AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
//SOFTWARE.

// Should add documentation in future and also watch asm_const in future
// Reference: https://dev.to/carey/embed-a-windows-manifest-in-your-rust-program-26j2
use std::arch::global_asm;

global_asm!(
    r#".section .rsrc$01,"dw""#,
    ".p2align 2",
    "2:",
    ".zero 14",
    ".short 1",
    ".long 24",
    ".long (3f - 2b) | 0x80000000",
    "3:",
    ".zero 14",
    ".short 1",
    ".long 1",
    ".long (4f - 2b) | 0x80000000",
    "4:",
    ".zero 14",
    ".short 1",
    ".long 1033",
    ".long 5f - 2b",
    "5:",
    ".long MANIFEST@imgrel",
    ".long 526", // Needs to match MANIFEST len()
    ".zero 8",
    ".p2align 2",
);

#[no_mangle]
#[link_section = ".rsrc$02"]
static mut MANIFEST: [u8; 526] = *br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
    <assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0" xmlns:asmv3="urn:schemas-microsoft-com:asm.v3">
      <asmv3:application>
        <asmv3:windowsSettings>
          <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true</dpiAware>
          <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
        </asmv3:windowsSettings>
      </asmv3:application>
    </assembly>"#;
