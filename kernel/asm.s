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

global switch_context
switch_context:  ; fn switch_context(next_ctx, current_ctx);
  mov [rsi + 0x40], rax
  mov [rsi + 0x48], rbx
  mov [rsi + 0x50], rcx
  mov [rsi + 0x58], rdx
  mov [rsi + 0x60], rdi
  mov [rsi + 0x68], rsi

  lea rax, [rsp + 8]
  mov [rsi + 0x70], rax  ; RSP
  mov [rsi + 0x78], rbp

  mov [rsi + 0x80], r8
  mov [rsi + 0x88], r9
  mov [rsi + 0x90], r10
  mov [rsi + 0x98], r11
  mov [rsi + 0xa0], r12
  mov [rsi + 0xa8], r13
  mov [rsi + 0xb0], r14
  mov [rsi + 0xb8], r15

  mov rax, cr3
  mov [rsi + 0x00], rax  ; CR3
  mov rax, [rsp]
  mov [rsi + 0x08], rax  ; RIP
  pushfq
  pop qword [rsi + 0x10] ; RFLAGS

  mov ax, cs
  mov [rsi + 0x20], rax
  mov bx, ss
  mov [rsi + 0x28], rbx
  mov cx, fs
  mov [rsi + 0x30], rcx
  mov dx, gs
  mov [rsi + 0x38], rdx

  fxsave [rsi + 0xc0]

  ; stack frame for iret
  push qword [rdi + 0x28] ; SS
  push qword [rdi + 0x70] ; RSP
  push qword [rdi + 0x10] ; RFLAGS
  push qword [rdi + 0x20] ; CS
  push qword [rdi + 0x08] ; RIP

  ; restore context
  fxrstor [rdi + 0xc0]

  mov rax, [rdi + 0x00]
  mov cr3, rax
  mov rax, [rdi + 0x30]
  mov fs, ax
  mov rax, [rdi + 0x38]
  mov gs, ax

  mov rax, [rdi + 0x40]
  mov rbx, [rdi + 0x48]
  mov rcx, [rdi + 0x50]
  mov rdx, [rdi + 0x58]
  mov rsi, [rdi + 0x68]
  mov rbp, [rdi + 0x78]
  mov r8,  [rdi + 0x80]
  mov r9,  [rdi + 0x88]
  mov r10, [rdi + 0x90]
  mov r11, [rdi + 0x98]
  mov r12, [rdi + 0xa0]
  mov r13, [rdi + 0xa8]
  mov r14, [rdi + 0xb0]
  mov r15, [rdi + 0xb8]

  mov rdi, [rdi + 0x60]

  o64 iret

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

global get_cr3 ; fn get_cr3() -> u64
get_cr3:
  mov rax, cr3
  ret

global set_cr3 ; fn set_cr3(u64)
set_cr3:
  mov cr3, rdi
  ret

; Syscall interrupt handler (int 0x80)
; Arguments are passed in: RAX (syscall number), RDI, RSI, RDX, R10, R8, R9
; Return value in RAX
extern syscall_entry
global syscall_handler_asm
syscall_handler_asm:
  ; Save callee-saved registers
  push rbx
  push rbp
  push r12
  push r13
  push r14
  push r15

  ; Build SyscallArgs struct on stack (must be 16-byte aligned)
  ; struct SyscallArgs { syscall_num, arg1, arg2, arg3, arg4, arg5, arg6 }
  sub rsp, 64            ; 7 * 8 = 56 bytes, rounded to 64 for alignment
  mov [rsp + 0], rax     ; syscall_num
  mov [rsp + 8], rdi     ; arg1
  mov [rsp + 16], rsi    ; arg2
  mov [rsp + 24], rdx    ; arg3
  mov [rsp + 32], r10    ; arg4
  mov [rsp + 40], r8     ; arg5
  mov [rsp + 48], r9     ; arg6

  ; Pass pointer to SyscallArgs as first argument
  mov rdi, rsp
  call syscall_entry

  ; Return value is already in RAX, save it in RBX (callee-saved)
  mov rbx, rax

  ; Clean up SyscallArgs
  add rsp, 64

  ; Restore callee-saved registers
  pop r15
  pop r14
  pop r13
  pop r12
  pop rbp

  ; Move return value to RAX (rbx will be popped next)
  mov rax, rbx
  pop rbx

  iretq

; Jump to user mode
; fn jump_to_user_mode(entry: u64, user_stack: u64, user_cs: u64, user_ss: u64)
; Arguments: rdi=entry, rsi=user_stack, rdx=user_cs, rcx=user_ss
global jump_to_user_mode
jump_to_user_mode:
  ; Disable interrupts while setting up
  cli

  ; Set up data segment registers for user mode
  mov ax, cx          ; user_ss
  mov ds, ax
  mov es, ax
  mov fs, ax
  mov gs, ax

  ; Build iretq stack frame
  push rcx            ; SS (user stack segment)
  push rsi            ; RSP (user stack pointer)
  push 0x202          ; RFLAGS (IF=1, reserved bit 1=1)
  push rdx            ; CS (user code segment)
  push rdi            ; RIP (entry point)

  ; Clear all general-purpose registers for clean start
  xor rax, rax
  xor rbx, rbx
  xor rcx, rcx
  xor rdx, rdx
  xor rsi, rsi
  xor rdi, rdi
  xor rbp, rbp
  xor r8, r8
  xor r9, r9
  xor r10, r10
  xor r11, r11
  xor r12, r12
  xor r13, r13
  xor r14, r14
  xor r15, r15

  ; Jump to user mode!
  iretq

; Jump to user mode with new page table
; fn jump_to_user_mode_with_cr3(entry: u64, user_stack: u64, user_cs: u64, user_ss: u64, cr3: u64)
; Arguments: rdi=entry, rsi=user_stack, rdx=user_cs, rcx=user_ss, r8=cr3
global jump_to_user_mode_with_cr3
jump_to_user_mode_with_cr3:
  ; Disable interrupts while setting up
  cli

  ; Set up the new page table first
  mov cr3, r8

  ; Set up data segment registers for user mode
  mov ax, cx          ; user_ss
  mov ds, ax
  mov es, ax
  mov fs, ax
  mov gs, ax

  ; Build iretq stack frame
  push rcx            ; SS (user stack segment)
  push rsi            ; RSP (user stack pointer)
  push 0x202          ; RFLAGS (IF=1, reserved bit 1=1)
  push rdx            ; CS (user code segment)
  push rdi            ; RIP (entry point)

  ; Clear all general-purpose registers for clean start
  xor rax, rax
  xor rbx, rbx
  xor rcx, rcx
  xor rdx, rdx
  xor rsi, rsi
  xor rdi, rdi
  xor rbp, rbp
  xor r8, r8
  xor r9, r9
  xor r10, r10
  xor r11, r11
  xor r12, r12
  xor r13, r13
  xor r14, r14
  xor r15, r15

  ; Jump to user mode!
  iretq
