use anchor_lang::prelude::*;

declare_id!("Ev5Fz9Fsy34PAdtEJqLLYRrFdR2oL3o1sE3pH151aiG7");

// ---- Config (safe under 10KB) ----
pub const MAX_TODO_LIST_LENGTH: usize = 40;   // total slots (live + holes)
pub const MAX_CONTENT_LEN: usize      = 200;  // bytes

// ---- Errors ----
#[error_code]
pub enum TodoError {
    #[msg("Todo not found")]
    TodoNotFound,
    #[msg("Content too long")]
    ContentTooLong,
    #[msg("List is full")]
    ListFull,
    #[msg("Index out of bounds")]
    IndexOob,
}

// ---- Data types ----
// NOTE: We keep these derives even with InitSpace because this struct is nested in an account.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, InitSpace)]
pub struct Todo {
    pub id: Pubkey,                         // 32
    #[max_len(MAX_CONTENT_LEN)]
    pub content: String,                    // 4 + MAX_CONTENT_LEN
    pub completed: bool,                    // 1
}

// ---- Account ----
#[account]
#[derive(InitSpace)]
pub struct TodoListAccountData {
    pub authority: Pubkey,                  // 32
    pub count: u16,                         // live items
    #[max_len(MAX_TODO_LIST_LENGTH)]
    pub deleted_indexes: Vec<u16>,          // 4 + 2*N
    #[max_len(MAX_TODO_LIST_LENGTH)]
    pub todos: Vec<Todo>,                   // 4 + N * Todo::INIT_SPACE
}

impl TodoListAccountData {
    pub fn get_todo_index(&self, id: Pubkey) -> Result<usize> {
        for (i, t) in self.todos.iter().enumerate() {
            if t.id == id { return Ok(i); }
        }
        err!(TodoError::TodoNotFound)
    }
}

// ---- Accounts ----
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + TodoListAccountData::INIT_SPACE,
        seeds = [b"todo_list", authority.key().as_ref()],
        bump
    )]
    pub list: Account<'info, TodoListAccountData>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Mutate<'info> {
    #[account(
        mut,
        seeds = [b"todo_list", authority.key().as_ref()],
        bump,
        has_one = authority
    )]
    pub list: Account<'info, TodoListAccountData>,
    pub authority: Signer<'info>,
}

// ---- Program ----
#[program]
pub mod anchor_todo {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let list = &mut ctx.accounts.list;
        list.authority = ctx.accounts.authority.key();
        list.count = 0;
        list.deleted_indexes = Vec::new();
        list.todos = Vec::new();
        Ok(())
    }

    pub fn add_todo(ctx: Context<Mutate>, id: Pubkey, content: String) -> Result<()> {
        require!(content.len() <= MAX_CONTENT_LEN, TodoError::ContentTooLong);

        let list = &mut ctx.accounts.list;
        let total_slots = list.todos.len() + list.deleted_indexes.len();
        require!(total_slots < MAX_TODO_LIST_LENGTH, TodoError::ListFull);

        let new_todo = Todo { id, content, completed: false };

        if let Some(slot) = list.deleted_indexes.pop() {
            let i = slot as usize;
            require!(i < list.todos.len(), TodoError::IndexOob);
            list.todos[i] = new_todo;
        } else {
            list.todos.push(new_todo);
        }

        list.count = list.count.checked_add(1).unwrap();
        Ok(())
    }

    pub fn mark_done(ctx: Context<Mutate>, id: Pubkey) -> Result<()> {
        let list = &mut ctx.accounts.list;
        let i = list.get_todo_index(id)?;
        list.todos[i].completed = true;
        Ok(())
    }

    pub fn update_content(ctx: Context<Mutate>, id: Pubkey, content: String) -> Result<()> {
        require!(content.len() <= MAX_CONTENT_LEN, TodoError::ContentTooLong);
        let list = &mut ctx.accounts.list;
        let i = list.get_todo_index(id)?;
        list.todos[i].content = content;
        Ok(())
    }

    /// Logical delete: scrub contents, mark slot reusable, decrement live count.
    pub fn delete_todo(ctx: Context<Mutate>, id: Pubkey) -> Result<()> {
        let list = &mut ctx.accounts.list;
        let i = list.get_todo_index(id)?;

        // Scrub to avoid leaving old content around.
        list.todos[i].content.clear();
        list.todos[i].completed = false;
        list.todos[i].id = Pubkey::default();

        require!(i <= u16::MAX as usize, TodoError::IndexOob);
        list.deleted_indexes.push(i as u16);
        list.count = list.count.checked_sub(1).unwrap();
        Ok(())
    }
}
