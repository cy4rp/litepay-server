use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};

pub async fn init_pool(database_url: &str) -> anyhow::Result<SqlitePool> {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;

    run_migrations(&pool).await?;
    Ok(pool)
}

async fn run_migrations(pool: &SqlitePool) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS wallets (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            admin_key TEXT NOT NULL UNIQUE,
            invoice_key TEXT NOT NULL UNIQUE,
            balance_msat INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS payments (
            checking_id TEXT PRIMARY KEY,
            wallet_id TEXT NOT NULL,
            bolt11 TEXT NOT NULL,
            payment_hash TEXT NOT NULL,
            amount_msat INTEGER NOT NULL,
            memo TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT 'pending',
            is_incoming INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (wallet_id) REFERENCES wallets(id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_payments_wallet_id ON payments(wallet_id)
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_payments_payment_hash ON payments(payment_hash)
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub mod wallets {
    use chrono::Utc;
    use sqlx::SqlitePool;
    use uuid::Uuid;

    use crate::models::wallet::{generate_api_key, Wallet};

    pub async fn create(pool: &SqlitePool, name: &str) -> Result<Wallet, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        let admin_key = generate_api_key();
        let invoice_key = generate_api_key();
        let now = Utc::now();

        sqlx::query(
            r#"INSERT INTO wallets (id, name, admin_key, invoice_key, balance_msat, created_at, updated_at)
               VALUES (?, ?, ?, ?, 0, ?, ?)"#,
        )
        .bind(&id)
        .bind(name)
        .bind(&admin_key)
        .bind(&invoice_key)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(pool)
        .await?;

        get_by_id(pool, &id).await
    }

    pub async fn get_by_id(pool: &SqlitePool, id: &str) -> Result<Wallet, sqlx::Error> {
        sqlx::query_as::<_, Wallet>("SELECT * FROM wallets WHERE id = ?")
            .bind(id)
            .fetch_one(pool)
            .await
    }

    pub async fn get_by_admin_key(pool: &SqlitePool, key: &str) -> Result<Wallet, sqlx::Error> {
        sqlx::query_as::<_, Wallet>("SELECT * FROM wallets WHERE admin_key = ?")
            .bind(key)
            .fetch_one(pool)
            .await
    }

    pub async fn get_by_invoice_key(pool: &SqlitePool, key: &str) -> Result<Wallet, sqlx::Error> {
        sqlx::query_as::<_, Wallet>("SELECT * FROM wallets WHERE invoice_key = ?")
            .bind(key)
            .fetch_one(pool)
            .await
    }

    pub async fn get_by_any_key(pool: &SqlitePool, key: &str) -> Result<Wallet, sqlx::Error> {
        sqlx::query_as::<_, Wallet>(
            "SELECT * FROM wallets WHERE admin_key = ? OR invoice_key = ?",
        )
        .bind(key)
        .bind(key)
        .fetch_one(pool)
        .await
    }

    pub async fn update_balance(
        pool: &SqlitePool,
        id: &str,
        delta_msat: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE wallets SET balance_msat = balance_msat + ?, updated_at = datetime('now')
               WHERE id = ?"#,
        )
        .bind(delta_msat)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn list(pool: &SqlitePool) -> Result<Vec<Wallet>, sqlx::Error> {
        sqlx::query_as::<_, Wallet>("SELECT * FROM wallets ORDER BY created_at DESC")
            .fetch_all(pool)
            .await
    }

    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM wallets WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
}

pub mod payments {
    use chrono::Utc;
    use sqlx::SqlitePool;
    use uuid::Uuid;

    use crate::models::payment::Payment;

    pub async fn create(
        pool: &SqlitePool,
        wallet_id: &str,
        bolt11: &str,
        payment_hash: &str,
        amount_msat: i64,
        memo: &str,
        is_incoming: bool,
    ) -> Result<Payment, sqlx::Error> {
        let checking_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        sqlx::query(
            r#"INSERT INTO payments
               (checking_id, wallet_id, bolt11, payment_hash, amount_msat, memo, status, is_incoming, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, 'pending', ?, ?, ?)"#,
        )
        .bind(&checking_id)
        .bind(wallet_id)
        .bind(bolt11)
        .bind(payment_hash)
        .bind(amount_msat)
        .bind(memo)
        .bind(is_incoming)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(pool)
        .await?;

        get_by_checking_id(pool, &checking_id).await
    }

    pub async fn get_by_checking_id(
        pool: &SqlitePool,
        checking_id: &str,
    ) -> Result<Payment, sqlx::Error> {
        sqlx::query_as::<_, Payment>("SELECT * FROM payments WHERE checking_id = ?")
            .bind(checking_id)
            .fetch_one(pool)
            .await
    }

    pub async fn get_by_payment_hash(
        pool: &SqlitePool,
        hash: &str,
    ) -> Result<Payment, sqlx::Error> {
        sqlx::query_as::<_, Payment>("SELECT * FROM payments WHERE payment_hash = ?")
            .bind(hash)
            .fetch_one(pool)
            .await
    }

    pub async fn update_status(
        pool: &SqlitePool,
        checking_id: &str,
        status: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE payments SET status = ?, updated_at = datetime('now')
               WHERE checking_id = ?"#,
        )
        .bind(status)
        .bind(checking_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn list_by_wallet(
        pool: &SqlitePool,
        wallet_id: &str,
    ) -> Result<Vec<Payment>, sqlx::Error> {
        sqlx::query_as::<_, Payment>(
            "SELECT * FROM payments WHERE wallet_id = ? ORDER BY created_at DESC",
        )
        .bind(wallet_id)
        .fetch_all(pool)
        .await
    }
}
