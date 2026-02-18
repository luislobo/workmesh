pub fn layout(title: &str, content: &str, refresh_ms: u64) -> String {
    format!(
        r#"<!doctype html>
<html lang=\"en\">
<head>
  <meta charset=\"utf-8\" />
  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />
  <title>{title} · WorkMesh Service</title>
  <link rel=\"stylesheet\" href=\"/assets/app.css\" />
</head>
<body data-refresh-ms=\"{refresh_ms}\">
  <header class=\"topbar\">
    <div class=\"brand\">WorkMesh Service</div>
    <nav>
      <a href=\"/\">Dashboard</a>
      <a href=\"/sessions\">Sessions</a>
      <a href=\"/workstreams\">Workstreams</a>
      <a href=\"/worktrees\">Worktrees</a>
      <a href=\"/repos\">Repos</a>
      <a href=\"/healthz\">Health</a>
    </nav>
    <form method=\"post\" action=\"/auth/logout\">
      <button type=\"submit\">Logout</button>
    </form>
  </header>
  <main>
    {content}
  </main>
  <footer>
    <span id=\"ws-state\">Realtime: connecting...</span>
  </footer>
  <script src=\"/assets/app.js\"></script>
</body>
</html>"#,
        title = escape_html(title),
        content = content,
        refresh_ms = refresh_ms
    )
}

pub fn login_page(message: Option<&str>, next: &str) -> String {
    let message_html = message
        .map(|msg| format!("<p class=\"error\">{}</p>", escape_html(msg)))
        .unwrap_or_default();

    format!(
        r#"<!doctype html>
<html lang=\"en\">
<head>
  <meta charset=\"utf-8\" />
  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />
  <title>Login · WorkMesh Service</title>
  <link rel=\"stylesheet\" href=\"/assets/app.css\" />
</head>
<body class=\"login\">
  <main class=\"login-card\">
    <h1>WorkMesh Service</h1>
    <p>Enter your access token to continue.</p>
    {message_html}
    <form method=\"post\" action=\"/auth/login\">
      <input type=\"hidden\" name=\"next\" value=\"{next}\" />
      <label for=\"token\">Token</label>
      <input id=\"token\" name=\"token\" type=\"password\" autocomplete=\"off\" required />
      <button type=\"submit\">Sign in</button>
    </form>
  </main>
</body>
</html>"#,
        message_html = message_html,
        next = escape_html(next)
    )
}

pub fn card(label: &str, value: impl ToString) -> String {
    format!(
        "<article class=\"card\"><h3>{}</h3><p>{}</p></article>",
        escape_html(label),
        escape_html(&value.to_string())
    )
}

pub fn section(title: &str, body: &str) -> String {
    format!("<section><h2>{}</h2>{}</section>", escape_html(title), body)
}

pub fn table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let header_html = headers
        .iter()
        .map(|h| format!("<th>{}</th>", escape_html(h)))
        .collect::<Vec<_>>()
        .join("");

    let row_html = rows
        .iter()
        .map(|row| {
            let cols = row
                .iter()
                .map(|col| format!("<td>{}</td>", escape_html(col)))
                .collect::<Vec<_>>()
                .join("");
            format!("<tr>{}</tr>", cols)
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "<div class=\"table-wrap\"><table><thead><tr>{}</tr></thead><tbody>{}</tbody></table></div>",
        header_html, row_html
    )
}

pub fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}
