use std::collections::HashMap;
use std::fmt;
use std::io::{self, BufRead, Write};

#[derive(Debug, PartialEq)]
enum LedgerError {
    AccountExists(String),
    AccountNotFound(String),
    InsufficientFunds { have: u64, need: u64 },
    Overflow,
    SameAccount,
    InvalidCommand(String),
}

impl fmt::Display for LedgerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LedgerError::AccountExists(n) => write!(f, "аккаунт '{}' уже существует", n),
            LedgerError::AccountNotFound(n) => write!(f, "аккаунт '{}' не найден", n),
            LedgerError::InsufficientFunds { have, need } => {
                write!(f, "недостаточно средств: есть {}, нужно {}", have, need)
            }
            LedgerError::Overflow => write!(f, "переполнение баланса"),
            LedgerError::SameAccount => write!(f, "нельзя переводить самому себе"),
            LedgerError::InvalidCommand(s) => write!(f, "неверная команда: {}", s),
        }
    }
}

#[derive(Debug)]
struct Account {
    balance: u64,
}

#[derive(Debug)]
enum Instruction {
    Create { name: String },
    Mint { name: String, amount: u64 },
    Transfer { from: String, to: String, amount: u64 },
    Balance { name: String },
}

impl Instruction {
    fn parse(line: &str) -> Result<Self, LedgerError> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        let parse_amount = |s: &str| {
            s.parse::<u64>()
                .map_err(|_| LedgerError::InvalidCommand(format!("'{}' не число", s)))
        };

        match parts.as_slice() {
            ["create", name] => Ok(Instruction::Create { name: name.to_string() }),
            ["mint", name, amount] => Ok(Instruction::Mint {
                name: name.to_string(),
                amount: parse_amount(amount)?,
            }),
            ["transfer", from, to, amount] => Ok(Instruction::Transfer {
                from: from.to_string(),
                to: to.to_string(),
                amount: parse_amount(amount)?,
            }),
            ["balance", name] => Ok(Instruction::Balance { name: name.to_string() }),
            _ => Err(LedgerError::InvalidCommand(line.to_string())),
        }
    }
}

struct Ledger {
    accounts: HashMap<String, Account>,
}

impl Ledger {
    fn new() -> Self {
        Ledger { accounts: HashMap::new() }
    }

    fn process(&mut self, ix: Instruction) -> Result<String, LedgerError> {
        match ix {
            Instruction::Create { name } => {
                if self.accounts.contains_key(&name) {
                    return Err(LedgerError::AccountExists(name));
                }
                self.accounts.insert(name.clone(), Account { balance: 0 });
                Ok(format!("аккаунт '{}' создан", name))
            }
            Instruction::Mint { name, amount } => {
                let acc = self
                    .accounts
                    .get_mut(&name)
                    .ok_or_else(|| LedgerError::AccountNotFound(name.clone()))?;
                acc.balance = acc.balance.checked_add(amount).ok_or(LedgerError::Overflow)?;
                Ok(format!("{}: баланс {}", name, acc.balance))
            }
            Instruction::Transfer { from, to, amount } => {
                if from == to {
                    return Err(LedgerError::SameAccount);
                }
                let from_bal = self
                    .accounts
                    .get(&from)
                    .ok_or_else(|| LedgerError::AccountNotFound(from.clone()))?
                    .balance;
                let to_bal = self
                    .accounts
                    .get(&to)
                    .ok_or_else(|| LedgerError::AccountNotFound(to.clone()))?
                    .balance;

                let new_from = from_bal.checked_sub(amount).ok_or(
                    LedgerError::InsufficientFunds { have: from_bal, need: amount },
                )?;
                let new_to = to_bal.checked_add(amount).ok_or(LedgerError::Overflow)?;

                // все проверки прошли — теперь меняем состояние
                self.accounts.get_mut(&from).unwrap().balance = new_from;
                self.accounts.get_mut(&to).unwrap().balance = new_to;
                Ok(format!("переведено {} от {} к {}", amount, from, to))
            }
            Instruction::Balance { name } => {
                let acc = self
                    .accounts
                    .get(&name)
                    .ok_or_else(|| LedgerError::AccountNotFound(name.clone()))?;
                Ok(format!("{}: {}", name, acc.balance))
            }
        }
    }
}

fn main() {
    let mut ledger = Ledger::new();
    println!("Команды: create <имя> | mint <имя> <сумма> | transfer <от> <кому> <сумма> | balance <имя> | exit");

    let stdin = io::stdin();
    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut line = String::new();
        if stdin.lock().read_line(&mut line).unwrap() == 0 {
            break;
        }
        let line = line.trim();
        if line == "exit" {
            break;
        }
        if line.is_empty() {
            continue;
        }

        let result = Instruction::parse(line).and_then(|ix| ledger.process(ix));
        match result {
            Ok(msg) => println!("✅ {}", msg),
            Err(e) => println!("❌ {}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transfer_works() {
        let mut l = Ledger::new();
        l.process(Instruction::Create { name: "a".into() }).unwrap();
        l.process(Instruction::Create { name: "b".into() }).unwrap();
        l.process(Instruction::Mint { name: "a".into(), amount: 100 }).unwrap();
        l.process(Instruction::Transfer { from: "a".into(), to: "b".into(), amount: 40 }).unwrap();
        assert_eq!(l.accounts["a"].balance, 60);
        assert_eq!(l.accounts["b"].balance, 40);
    }

    #[test]
    fn insufficient_funds() {
        let mut l = Ledger::new();
        l.process(Instruction::Create { name: "a".into() }).unwrap();
        l.process(Instruction::Create { name: "b".into() }).unwrap();
        let r = l.process(Instruction::Transfer { from: "a".into(), to: "b".into(), amount: 1 });
        assert_eq!(r, Err(LedgerError::InsufficientFunds { have: 0, need: 1 }));
    }
}