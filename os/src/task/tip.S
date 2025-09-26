.section .text.tip
.globl __tip

__tip:
    # 函数序（Prologue）：分配栈空间并保存寄存器
    addi sp, sp, -16    # 为栈帧分配16字节空间（RV64下，每个寄存器占8字节）
    sd ra, 8(sp)        # 保存返回地址寄存器ra（caller-saved）
    # 注：原代码"sd spe"可能存在笔误，此处假设需保存寄存器s0（常见callee-saved寄存器）
    sd s0, 0(sp)        # 保存寄存器s0（若需使用其他保存寄存器，如s1-s11，需同样处理）

    # 函数体：调用tip函数
    call tip            # 调用tip函数（会覆盖ra寄存器）

    # 函数结（Epilogue）：恢复寄存器并释放栈空间
    ld s0, 0(sp)        # 恢复s0寄存器
    ld ra, 8(sp)        # 恢复返回地址ra
    addi sp, sp, 16     # 释放栈空间
    ret                 # 返回调用者（使用ra中地址）