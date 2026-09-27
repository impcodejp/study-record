//! リポジトリ（テーブルごとの SQL をまとめたもの）。
//!
//! すべての関数は `&mut SqliteConnection` を受け取る。
//! トランザクションの範囲はアプリケーション層が決める。

pub mod master_repository;
pub mod practice_repository;
pub mod user_repository;
