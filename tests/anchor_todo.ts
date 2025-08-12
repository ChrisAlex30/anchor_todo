import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { AnchorTodo } from "../target/types/anchor_todo";
import { assert } from "chai";

// In sync with Rust constants
const MAX_TODO_LIST_LENGTH = 40;
const MAX_CONTENT_LEN = 200;

describe("anchor_todo", () => {
  const provider = anchor.AnchorProvider.local();
  anchor.setProvider(provider);
  const program = anchor.workspace.AnchorTodo as Program<AnchorTodo>;

  const todoAccount = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("todo_list"), provider.wallet.publicKey.toBuffer()],
    program.programId
  )[0];

  

  it("Initialize", async () => {

    const info = await provider.connection.getAccountInfo(todoAccount);
    if (!info) {
    await program.methods
      .initialize()
      .accounts({        
        authority: provider.wallet.publicKey,        
      })
      .rpc();
    }

    const list = await program.account.todoListAccountData.fetch(todoAccount);
    console.log({list});
    
    assert(list.authority.equals(provider.wallet.publicKey));
    assert.strictEqual(Number(list.count), 0);
    assert.strictEqual(list.todos.length, 0);
    assert.strictEqual(list.deletedIndexes.length, 0);
  });

  it("Add → MarkDone → UpdateContent → Delete (slot reuse)", async () => {
    const id = anchor.web3.PublicKey.unique();

    // Add
    await program.methods
      .addTodo(id, "Write tests")
      .accounts(
        {
          list:todoAccount,
        }
      )
      .rpc();

    let list = await program.account.todoListAccountData.fetch(todoAccount);
    console.log({list});
    assert(list.authority.equals(provider.wallet.publicKey));
    assert.strictEqual(Number(list.count), 1);
    let i1 = list.todos.findIndex((t) => t.id.equals(id));
    assert.isAtLeast(i1,0);
    assert.strictEqual(list.todos[i1].completed,false);
    assert.strictEqual(list.todos[i1].content,"Write tests");    
    assert.strictEqual(list.deletedIndexes.length, 0);


  // Mark done
    await program.methods
      .markDone(id)
      .accounts({
        list:todoAccount
      })
      .rpc();

    list = await program.account.todoListAccountData.fetch(todoAccount);
    console.log({list});
    assert(list.authority.equals(provider.wallet.publicKey));
    i1 = list.todos.findIndex((t) => t.id.equals(id));
    assert.strictEqual(list.todos[i1].completed,true);


    // Update content
    await program.methods
      .updateContent(id, "Write more tests")
      .accounts({ list: todoAccount})
      .rpc();

    list = await program.account.todoListAccountData.fetch(todoAccount);
    i1 = list.todos.findIndex((t: any) => t.id.equals(id));
    assert.strictEqual(list.todos[i1].content,"Write more tests");

  // Delete (logical) → pushes index into free list
    await program.methods
      .deleteTodo(id)
      .accounts({ list: todoAccount})
      .rpc();

    list = await program.account.todoListAccountData.fetch(todoAccount);
    assert.strictEqual(Number(list.count), 0);
    assert.strictEqual(list.deletedIndexes.length, 1); // free-list gained one
    assert.strictEqual(list.todos.length, 1); // vec length unchanged

    //Reusing the deleted slot
    const id2 = anchor.web3.PublicKey.unique();
    await program.methods
      .addTodo(id2, "Reused slot")
      .accounts({ list: todoAccount })
      .rpc();

    list = await program.account.todoListAccountData.fetch(todoAccount);
    assert.strictEqual(Number(list.count), 1);

    const i2 = list.todos.findIndex((t: any) => t.id.equals(id2));
    // Reused the hole (i2 should equal i1)
    assert.strictEqual(i2, i1);
    assert.strictEqual(list.todos[i2].completed,false);
    assert.strictEqual(list.todos[i2].content,"Reused slot");
    assert.strictEqual(list.todos.length, 1); // still no growth
  });








 

  
});