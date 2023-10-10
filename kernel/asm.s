; System V AMD64 Calling Convention

bits 64

extern kernel_main_virt

section .bss align=16
kernel_main_stack:
  resb 1024 * 1024

section .text
global kernel_main
kernel_main:
  mov rsp, kernel_main_stack + 1024 * 1024
  call kernel_main_virt
.fin:
  hlt
  jmp .fin

global load_gdt ; fn load_gdt(limit: u16, offset: usize)
load_gdt:
  push rbp
  mov rbp, rsp
  sub rsp, 10
  mov [rsp], di ; limit
  mov [rsp + 2], rsi ; offset
  lgdt [rsp]
  mov rsp, rbp
  pop rbp
  ret

global set_ds_all ; fn set_ds_all(value: u64)
set_ds_all:
  mov ds, di
  mov es, di
  mov fs, di
  mov gs, di
  ret

global set_cs_ss ; fn set_cs_ss(cs: u16, ss: u16)
set_cs_ss:
  push rbp
  mov rbp, rsp
  mov ss, si
  mov rax, .next
  push rdi
  push rax
  o64 retf
.next:
  mov rsp, rbp
  pop rbp
  ret

global set_cr3 ; fn set_cr3(u64)
set_cr3:
  mov cr3, rdi
  ret
