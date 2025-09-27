ch2_bad_address在u态访问0x0会被mmu拦截造成非法访问
 ch2b_bad_instructions在u态执行s态特权指令 sret会被cpu拦截产生非法指令异常，
 ch2b_bad_register在u态访问csr寄存器，cpu产生非法访问异常 
 trap.S中__alltraps用于保存除了spec等cpu自动保存的寄存器到任务上下文（快照） __restore用于恢复任务上下文sret返回u态恢复任务，刚进入 __restore 时，sp 代表该应用内核栈指针
，L43-L48：这几行汇编代码特殊处理了sstatus，spec，sscratch，t0,t1,t2寄存器，保证函数正确传参处理，并且保证任务可以正确返回，
L50-L56：为何跳过了 x2 和 x4？因为x4是tp线程指针，目前用不到，x2是sp最后在sscrach保存，全程有动态作用，无需在trapcontext保存，
csrrw sp, sscratch, sp后sscratch和sp会交互，达到原子操作直接切换用户栈和内核栈。__restore：中发生状态切换在哪一条指令？发生在sret，是一条cpu特权指令，特权级将为u，pc跳转到spec。
 U 态进入 S 态是哪一条指令发生的？ecall