//! HTTP API を通した結合テスト。
//!
//! 一時フォルダに SQLite を作り、実際のルーターに要求を送って業務ルールを確認する。

use std::path::PathBuf;

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

use study_record_server::{
    application::startup_service,
    build_state,
    domain::time::now_utc_string,
    infrastructure::{
        config::AppConfig,
        database,
        repositories::user_repository,
        security::hash_password,
    },
    presentation::router,
};

/// テスト用のアプリ。
struct TestApp {
    router: Router,
    pool: sqlx::SqlitePool,
    dir: PathBuf,
}

impl Drop for TestApp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// ログイン済みのクライアント。
struct Client {
    cookie: String,
    csrf: String,
}

/// テスト用のアプリを作る（DB は一時フォルダ）。
async fn setup(name: &str) -> TestApp {
    let dir = std::env::temp_dir().join(format!("study-record-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::env::set_var("APP_CONFIG", dir.join("none.ini"));
    let config = AppConfig::load().unwrap();
    let pool = database::connect(&dir.join("test.sqlite3")).await.unwrap();
    startup_service::run(&pool).await.unwrap();
    let state = build_state(&config, pool.clone()).unwrap();
    TestApp {
        router: router::build(state),
        pool,
        dir,
    }
}

/// ユーザーを直接 DB に作る（メール確認を省略するため）。
async fn create_user(app: &TestApp, email: &str, password: &str) {
    let mut conn = app.pool.acquire().await.unwrap();
    let hash = hash_password(password).unwrap();
    user_repository::insert(&mut conn, "テスト", email, &hash, &now_utc_string())
        .await
        .unwrap();
}

/// 要求を送り、ステータスと JSON（または文字列）を返す。
async fn send(
    app: &TestApp,
    method: &str,
    uri: &str,
    client: Option<&Client>,
    body: Option<Value>,
) -> (StatusCode, Value, Option<String>) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("x-requested-with", "fetch");
    if let Some(client) = client {
        builder = builder
            .header(header::COOKIE, &client.cookie)
            .header("x-csrf-token", &client.csrf);
    }
    let request = match body {
        Some(body) => builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    };
    let response = app.router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let set_cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .map(|v| v.to_str().unwrap().to_string());
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()));
    (status, value, set_cookie)
}

/// ログインしてクライアントを返す。
async fn login(app: &TestApp, email: &str, password: &str) -> Client {
    let (status, body, cookie) = send(
        app,
        "POST",
        "/api/auth/login",
        None,
        Some(json!({ "email": email, "password": password })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let cookie = cookie.unwrap().split(';').next().unwrap().to_string();
    Client {
        cookie,
        csrf: body["csrfToken"].as_str().unwrap().to_string(),
    }
}

/// 回答を記録する要求の本文。
fn answer(title: &str, number: i64, area: &str, response: &str) -> Value {
    json!({ "title": title, "questionNumber": number, "area": area, "response": response, "elapsedSeconds": 30 })
}

#[tokio::test]
async fn health_is_public() {
    let app = setup("health").await;
    let (status, body, _) = send(&app, "GET", "/api/health", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, Value::String("ok".into()));
}

#[tokio::test]
async fn full_learning_flow() {
    let app = setup("flow").await;
    create_user(&app, "a@example.com", "password-1").await;
    let c = login(&app, "a@example.com", "password-1").await;

    // ユーザー情報
    let (status, me, _) = send(&app, "GET", "/api/auth/me", Some(&c), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me["user"]["email"], "a@example.com");

    // 試験の登録（「未分類」カテゴリが自動で作られる）
    let (status, exams, _) =
        send(&app, "POST", "/api/exams", Some(&c), Some(json!({ "name": "簿記2級" }))).await;
    assert_eq!(status, StatusCode::OK, "{exams}");
    let exam_id = exams[0]["id"].as_i64().unwrap();
    let (_, dup, _) =
        send(&app, "POST", "/api/exams", Some(&c), Some(json!({ "name": "簿記2級" }))).await;
    assert_eq!(dup["error"], "同じ試験名が既にあります。");

    let base = format!("/api/exams/{exam_id}");
    let (_, cats, _) = send(&app, "POST", &format!("{base}/categories"), Some(&c), Some(json!({ "name": "仕訳" }))).await;
    assert_eq!(cats.as_array().unwrap().len(), 2);
    assert_eq!(cats[0]["name"], "未分類");
    let shiwake_id = cats[1]["id"].as_i64().unwrap();

    // 学習の開始（同時に 1 つだけ）
    let (status, practice, _) =
        send(&app, "POST", &format!("{base}/practices"), Some(&c), Some(json!({ "title": " 第1回 " }))).await;
    assert_eq!(status, StatusCode::OK, "{practice}");
    assert_eq!(practice["title"], "第1回");
    let pid = practice["id"].as_i64().unwrap();
    let (status, err, _) =
        send(&app, "POST", &format!("{base}/practices"), Some(&c), Some(json!({ "title": "x" }))).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(err["error"], "進行中の学習があります。");

    // 回答の記録（自由記述・改行可）
    let answers_uri = format!("/api/practices/{pid}/answers");
    let (status, p, _) = send(&app, "POST", &answers_uri, Some(&c), Some(answer("第1回", 1, "仕訳", "借方 現金\r\n貸方 売上"))).await;
    assert_eq!(status, StatusCode::OK, "{p}");
    assert_eq!(p["answers"][0]["response"], "借方 現金\n貸方 売上");
    let (status, err, _) = send(&app, "POST", &answers_uri, Some(&c), Some(answer("第1回", 1, "仕訳", "x"))).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(err["error"], "この問題数はすでに記録されています。");
    let (status, err, _) = send(&app, "POST", &answers_uri, Some(&c), Some(answer("第1回", 2, "存在しない", "x"))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(err["error"], "カテゴリを選択してください。");
    let (_, p, _) = send(&app, "POST", &answers_uri, Some(&c), Some(answer("第1回", 2, "未分類", "ア"))).await;
    let a1 = p["answers"][0]["id"].as_i64().unwrap();
    let a2 = p["answers"][1]["id"].as_i64().unwrap();

    // 進行中の学習の取得
    let (_, active, _) = send(&app, "GET", "/api/active", Some(&c), None).await;
    assert_eq!(active["id"].as_i64(), Some(pid));

    // 未採点の回答は修正できる
    let (status, p, _) = send(&app, "PUT", &format!("{answers_uri}/{a2}"), Some(&c), Some(answer("第1回", 2, "仕訳", "イ"))).await;
    assert_eq!(status, StatusCode::OK, "{p}");
    assert_eq!(p["answers"][1]["response"], "イ");

    // 採点は記録順
    let (status, err, _) = send(&app, "POST", &format!("{answers_uri}/{a2}/grade"), Some(&c), Some(json!({ "correct": true }))).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(err["error"], "記録順に採点してください。");
    let (status, err, _) = send(&app, "POST", &format!("/api/practices/{pid}/finish"), Some(&c), None).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(err["error"], "すべての回答を採点してください。");
    let (_, p, _) = send(&app, "POST", &format!("{answers_uri}/{a1}/grade"), Some(&c),
        Some(json!({ "correct": false, "correctAnswer": "借方 売掛金", "note": "掛売上に注意" }))).await;
    assert!(p["completedAt"].is_null());
    let (_, p, _) = send(&app, "POST", &format!("{answers_uri}/{a2}/grade"), Some(&c), Some(json!({ "correct": true }))).await;
    assert!(p["completedAt"].is_string(), "最後の採点で自動的に完了する");
    assert_eq!(p["correctCount"], 1);

    // 完了済みの学習には追加できない
    let (status, err, _) = send(&app, "POST", &answers_uri, Some(&c), Some(answer("第1回", 3, "仕訳", "ウ"))).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(err["error"], "この回の採点は終了しています。");
    let (_, active, _) = send(&app, "GET", "/api/active", Some(&c), None).await;
    assert!(active.is_null());

    // ダッシュボード
    let (_, dash, _) = send(&app, "GET", &format!("{base}/dashboard"), Some(&c), None).await;
    assert_eq!(dash["totalCount"], 2);
    assert_eq!(dash["correctCount"], 1);
    assert_eq!(dash["averageSeconds"], 30.0);
    assert_eq!(dash["dailyCounts"].as_array().unwrap().len(), 14);
    assert_eq!(dash["dailyCounts"][13]["count"], 2);
    assert_eq!(dash["areaStats"][0]["area"], "未分類");
    assert_eq!(dash["areaStats"][0]["totalCount"], 0);
    assert_eq!(dash["areaStats"][1]["totalCount"], 2);

    // 問題一覧・復習一覧・カテゴリ別
    let (_, qs, _) = send(&app, "GET", &format!("{base}/questions"), Some(&c), None).await;
    assert_eq!(qs.as_array().unwrap().len(), 2);
    let (_, review, _) = send(&app, "GET", &format!("{base}/questions?filter=review"), Some(&c), None).await;
    assert_eq!(review.as_array().unwrap().len(), 1);
    assert_eq!(review[0]["questionNumber"], 1);
    let (_, by_cat, _) = send(&app, "GET", &format!("{base}/questions?categoryId={shiwake_id}"), Some(&c), None).await;
    assert_eq!(by_cat.as_array().unwrap().len(), 2);

    // 正誤履歴（正解・メモ付き）
    let title = "%E7%AC%AC1%E5%9B%9E"; // 第1回
    let (_, hist, _) = send(&app, "GET", &format!("{base}/history?title={title}&questionNumber=1"), Some(&c), None).await;
    assert_eq!(hist[0]["correctAnswer"], "借方 売掛金");
    assert_eq!(hist[0]["note"], "掛売上に注意");

    // 採点済み回答のカテゴリ変更・メモ更新
    let cats = send(&app, "GET", &format!("{base}/categories"), Some(&c), None).await.1;
    let misc_id = cats[0]["id"].as_i64().unwrap();
    let (status, _, _) = send(&app, "PUT", &format!("/api/answers/{a1}/category"), Some(&c), Some(json!({ "categoryId": misc_id }))).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _, _) = send(&app, "PUT", &format!("/api/answers/{a1}/memo"), Some(&c), Some(json!({ "note": "更新" }))).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // カテゴリ名の変更は回答にも反映され、使用中のカテゴリは削除できない
    let (_, cats, _) = send(&app, "PUT", &format!("{base}/categories/{shiwake_id}"), Some(&c), Some(json!({ "name": "仕訳問題" }))).await;
    assert_eq!(cats[1]["name"], "仕訳問題");
    assert_eq!(cats[1]["answerCount"], 1);
    let (status, err, _) = send(&app, "DELETE", &format!("{base}/categories/{shiwake_id}"), Some(&c), None).await;
    assert_eq!(status, StatusCode::CONFLICT, "{err}");

    // 学習履歴一覧・CSV
    let (_, list, _) = send(&app, "GET", &format!("{base}/practices"), Some(&c), None).await;
    assert_eq!(list[0]["questionCount"], 2);
    let (status, csv, _) = send(&app, "GET", &format!("{base}/export"), Some(&c), None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(csv.as_str().unwrap().contains("\"借方 現金\n貸方 売上\""));

    // 学習記録がある試験は削除できない
    let (status, _, _) = send(&app, "DELETE", &base, Some(&c), None).await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn abort_keeps_only_graded_answers() {
    let app = setup("abort").await;
    create_user(&app, "b@example.com", "password-1").await;
    let c = login(&app, "b@example.com", "password-1").await;
    let exams = send(&app, "POST", "/api/exams", Some(&c), Some(json!({ "name": "応用情報" }))).await.1;
    let base = format!("/api/exams/{}", exams[0]["id"]);

    // 回答なしで中断すると学習ごと消える
    let p = send(&app, "POST", &format!("{base}/practices"), Some(&c), Some(json!({ "title": "午前" }))).await.1;
    let (_, result, _) = send(&app, "POST", &format!("/api/practices/{}/abort", p["id"]), Some(&c), None).await;
    assert!(result["practice"].is_null());

    // 採点済みが残る場合は完了扱い
    let p = send(&app, "POST", &format!("{base}/practices"), Some(&c), Some(json!({ "title": "午前" }))).await.1;
    let pid = p["id"].as_i64().unwrap();
    let uri = format!("/api/practices/{pid}/answers");
    let p = send(&app, "POST", &uri, Some(&c), Some(answer("午前", 1, "未分類", "ア"))).await.1;
    send(&app, "POST", &uri, Some(&c), Some(answer("午前", 2, "未分類", "イ"))).await;
    let a1 = p["answers"][0]["id"].as_i64().unwrap();
    send(&app, "POST", &format!("{uri}/{a1}/grade"), Some(&c), Some(json!({ "correct": true }))).await;
    let (_, result, _) = send(&app, "POST", &format!("/api/practices/{pid}/abort"), Some(&c), None).await;
    assert_eq!(result["practice"]["questionCount"], 1);
    assert!(result["practice"]["completedAt"].is_string());
}

#[tokio::test]
async fn security_rules() {
    let app = setup("security").await;
    create_user(&app, "c@example.com", "password-1").await;
    create_user(&app, "d@example.com", "password-2").await;
    let c = login(&app, "c@example.com", "password-1").await;
    let d = login(&app, "d@example.com", "password-2").await;

    // 未ログイン
    let (status, _, _) = send(&app, "GET", "/api/active", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // CSRF トークンが違う
    let bad = Client { cookie: c.cookie.clone(), csrf: "wrong".into() };
    let (status, _, _) = send(&app, "POST", "/api/exams", Some(&bad), Some(json!({ "name": "x" }))).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // X-Requested-With が無い
    let request = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "email": "c@example.com", "password": "password-1" }).to_string()))
        .unwrap();
    let response = app.router.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // 他人の試験は見えない
    let exams = send(&app, "POST", "/api/exams", Some(&c), Some(json!({ "name": "秘密" }))).await.1;
    let exam_id = exams[0]["id"].as_i64().unwrap();
    let (status, _, _) = send(&app, "GET", &format!("/api/exams/{exam_id}/dashboard"), Some(&d), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // ログイン失敗が続くとロックされる
    for _ in 0..5 {
        let (status, _, _) = send(&app, "POST", "/api/auth/login", None,
            Some(json!({ "email": "d@example.com", "password": "wrong-pass" }))).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
    let (status, _, _) = send(&app, "POST", "/api/auth/login", None,
        Some(json!({ "email": "d@example.com", "password": "password-2" }))).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);

    // メール未設定なら登録系は 503
    let (status, _, _) = send(&app, "POST", "/api/auth/register", None,
        Some(json!({ "name": "新規", "email": "new@example.com" }))).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);

    // ログアウト後はセッションが無効
    let (status, _, _) = send(&app, "POST", "/api/auth/logout", Some(&c), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _, _) = send(&app, "GET", "/api/auth/me", Some(&c), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn study_plan_flow() {
    let app = setup("plan").await;
    create_user(&app, "plan@example.com", "password-1").await;
    let c = login(&app, "plan@example.com", "password-1").await;
    let (_, exams, _) = send(&app, "POST", "/api/exams", Some(&c), Some(json!({ "name": "応用情報" }))).await;
    let exam_id = exams[0]["id"].as_i64().unwrap();
    let base = format!("/api/exams/{exam_id}");
    let today = study_record_server::domain::time::today_jst();
    let exam_day = (today + chrono::Duration::days(30)).format("%Y-%m-%d").to_string();

    // 学習目標の設定（試験日・1 日の目標）。範囲外はエラー
    let (status, exams, _) = send(&app, "PUT", &format!("{base}/goal"), Some(&c),
        Some(json!({ "examDate": exam_day, "dailyGoal": 10 }))).await;
    assert_eq!(status, StatusCode::OK, "{exams}");
    assert_eq!(exams[0]["examDate"], exam_day.as_str());
    assert_eq!(exams[0]["dailyGoal"], 10);
    let (status, err, _) = send(&app, "PUT", &format!("{base}/goal"), Some(&c), Some(json!({ "dailyGoal": 0 }))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(err["error"], "1日の目標は1～1000問で入力してください。");
    let (status, _, _) = send(&app, "PUT", &format!("{base}/goal"), Some(&c), Some(json!({ "examDate": "2026/10/18" }))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // 問1 は不正解、問2 は正解
    let (_, p, _) = send(&app, "POST", &format!("{base}/practices"), Some(&c), Some(json!({ "title": "午前" }))).await;
    let pid = p["id"].as_i64().unwrap();
    let answers_uri = format!("/api/practices/{pid}/answers");
    send(&app, "POST", &answers_uri, Some(&c), Some(answer("午前", 1, "未分類", "ア"))).await;
    let (_, p, _) = send(&app, "POST", &answers_uri, Some(&c), Some(answer("午前", 2, "未分類", "イ"))).await;
    // 問1 は不正解（メモを残す）、問2 は正解
    let first = p["answers"][0]["id"].as_i64().unwrap();
    let second = p["answers"][1]["id"].as_i64().unwrap();
    send(&app, "POST", &format!("{answers_uri}/{first}/grade"), Some(&c),
        Some(json!({ "correct": false, "correctAnswer": "ウ", "note": "単位の換算を忘れた" }))).await;
    send(&app, "POST", &format!("{answers_uri}/{second}/grade"), Some(&c), Some(json!({ "correct": true }))).await;

    // 見直しノート：メモを残した回答だけ
    let (status, notes, _) = send(&app, "GET", &format!("{base}/notes"), Some(&c), None).await;
    assert_eq!(status, StatusCode::OK, "{notes}");
    let notes = notes.as_array().unwrap();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0]["questionNumber"], 1);
    assert_eq!(notes[0]["note"], "単位の換算を忘れた");
    assert_eq!(notes[0]["correctAnswer"], "ウ");

    // 今日の状況：目標 10 問中 2 問、連続 1 日、復習はまだ来ていない
    let (_, dash, _) = send(&app, "GET", &format!("{base}/dashboard"), Some(&c), None).await;
    let t = &dash["today"];
    assert_eq!(t["daysUntilExam"], 30);
    assert_eq!(t["dailyGoal"], 10);
    assert_eq!(t["answeredCount"], 2);
    assert_eq!(t["correctCount"], 1);
    assert_eq!(t["currentStreak"], 1);
    assert_eq!(t["dueCount"], 0);
    assert_eq!(t["questionCount"], 2);
    // 準備度：習熟度 0 と 1 の平均 → 1/10 = 10%。習得まであと 5+4=9 回の正解、30 日あるので 1 日 1 問
    let r = &dash["readiness"];
    assert_eq!(r["percent"], 10.0);
    assert_eq!(r["remainingCorrectAnswers"], 9);
    assert_eq!(r["requiredDaily"], 1);
    assert_eq!(r["areas"][0]["area"], "未分類");
    assert_eq!(r["areas"][0]["questionCount"], 2);
    assert!((r["recentDailyAverage"].as_f64().unwrap() - 2.0 / 14.0).abs() < 1e-9);
    // 平均回答時間
    let (_, qs_all, _) = send(&app, "GET", &format!("{base}/questions"), Some(&c), None).await;
    assert_eq!(qs_all[0]["averageSeconds"], 30.0);
    // 他人の見直しノートは見られない
    create_user(&app, "peek@example.com", "password-3").await;
    let peek = login(&app, "peek@example.com", "password-3").await;
    let (status, _, _) = send(&app, "GET", &format!("{base}/notes"), Some(&peek), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(dash["heatmap"].as_array().unwrap().len(), 182);
    assert_eq!(dash["heatmap"][181]["count"], 2);
    let (_, qs, _) = send(&app, "GET", &format!("{base}/questions"), Some(&c), None).await;
    let tomorrow = (today + chrono::Duration::days(1)).format("%Y-%m-%d").to_string();
    let q1 = qs.as_array().unwrap().iter().find(|q| q["questionNumber"] == 1).unwrap();
    assert_eq!(q1["masteryLevel"], 0);
    assert_eq!(q1["nextReviewOn"], tomorrow.as_str());

    // 5 日前に解いたことにすると、両方とも復習の時期が来る（過ぎた日数が長い問1 が先）
    sqlx::query("UPDATE answers SET answered_at = datetime(answered_at, '-5 days')")
        .execute(&app.pool)
        .await
        .unwrap();
    let (status, due, _) = send(&app, "GET", &format!("{base}/questions?filter=due"), Some(&c), None).await;
    assert_eq!(status, StatusCode::OK, "{due}");
    let due = due.as_array().unwrap();
    assert_eq!(due.len(), 2);
    assert_eq!(due[0]["questionNumber"], 1);
    assert_eq!(due[1]["questionNumber"], 2);
    assert_eq!(due[1]["masteryLevel"], 1);
    let (_, dash, _) = send(&app, "GET", &format!("{base}/dashboard"), Some(&c), None).await;
    assert_eq!(dash["today"]["dueCount"], 2);
    assert_eq!(dash["today"]["answeredCount"], 0);
    assert_eq!(dash["today"]["currentStreak"], 0, "5 日空いたので連続記録は途切れる");
    assert_eq!(dash["today"]["longestStreak"], 1);

    // 目標を空にすると未設定に戻る
    let (_, exams, _) = send(&app, "PUT", &format!("{base}/goal"), Some(&c), Some(json!({}))).await;
    assert!(exams[0]["examDate"].is_null());
    assert!(exams[0]["dailyGoal"].is_null());

    // 他人の試験の目標は変えられない
    create_user(&app, "other@example.com", "password-2").await;
    let o = login(&app, "other@example.com", "password-2").await;
    let (status, _, _) = send(&app, "PUT", &format!("{base}/goal"), Some(&o), Some(json!({ "dailyGoal": 5 }))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn exam_templates_flow() {
    let app = setup("templates").await;

    // テンプレートの一覧はログインしなくても取れる（紹介ページで使う）
    let (status, templates, _) = send(&app, "GET", "/api/exam-templates", None, None).await;
    assert_eq!(status, StatusCode::OK, "{templates}");
    let takken = templates.as_array().unwrap().iter().find(|t| t["key"] == "takken").unwrap();
    assert_eq!(takken["name"], "宅地建物取引士");

    // テンプレートを指定して試験を作ると、「未分類」の後にテンプレートのカテゴリがそろう
    create_user(&app, "tpl@example.com", "password-1").await;
    let c = login(&app, "tpl@example.com", "password-1").await;
    let (status, exams, _) = send(&app, "POST", "/api/exams", Some(&c),
        Some(json!({ "name": "宅建 2026", "templateKey": "takken" }))).await;
    assert_eq!(status, StatusCode::OK, "{exams}");
    let exam_id = exams[0]["id"].as_i64().unwrap();
    let (_, cats, _) = send(&app, "GET", &format!("/api/exams/{exam_id}/categories"), Some(&c), None).await;
    let names: Vec<&str> = cats.as_array().unwrap().iter().map(|c| c["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["未分類", "権利関係", "法令上の制限", "宅建業法", "税・その他"]);

    // 存在しないテンプレートはエラー（試験も作られない）
    let (status, err, _) = send(&app, "POST", "/api/exams", Some(&c),
        Some(json!({ "name": "x", "templateKey": "nothing" }))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(err["error"], "試験のテンプレートが見つかりません。");
    let (_, exams, _) = send(&app, "GET", "/api/exams", Some(&c), None).await;
    assert_eq!(exams.as_array().unwrap().len(), 1);

    // 準備度の推移は 8 週分（最後が今日）
    let (_, dash, _) = send(&app, "GET", &format!("/api/exams/{exam_id}/dashboard"), Some(&c), None).await;
    let history = dash["readiness"]["history"].as_array().unwrap();
    assert_eq!(history.len(), 8);
    assert_eq!(
        history[7]["date"],
        study_record_server::domain::time::today_jst().format("%Y-%m-%d").to_string()
    );
}
