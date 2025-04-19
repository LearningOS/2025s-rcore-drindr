# 功能总结：
完成sys_trace对应功能，返回id地址处的值，将data的最低位写入id地址，通过`TaskManager`记录系统调用次数

# 问答题
1. bad(RustSBI version 0.3.0-alpha.2, adapting to RISC-V SBI v1.0.0)
  1. bad_address:
    [kernel] PageFault in application, bad addr = 0x0, bad instruction = 0x804003a4, kernel killed it.
  2. bad_instruction:
    [kernel] IllegalInstruction in application, kernel killed it.
  3. bad_register:
    [kernel] IllegalInstruction in application, kernel killed it.
2. trap.S
  1. 刚进入`__resotre`时`sp`是kernel stack指针。使用情景
     - 载入app
     - 从trap中恢复
  2. 从栈恢复sstatus，sepc，sscratch寄存器，sstatus是当前特权级状态，sepc是app的entry，sscratch用来存储用户态的栈指针
  3. x2是sp寄存器，此处指向内核栈，用户栈的指针在sscratch中；x4此处不会用到
  4. sp和sscratch交换，执行后sp变成用户栈指针，sscratch变为内核栈指针
  5. sret，该指令从内核态返回到用户态
  6. sp和sscratch交换，执行后sp变成内核栈指针，sscratch变为用户栈指针
  7. ecall从用户态进入内核态，在进入stvec中设置好的__alltraps
  
  
  

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

         无

2. 此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

        https://five-embeddev.com/quickref/regs_abi.html

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。
  
  
