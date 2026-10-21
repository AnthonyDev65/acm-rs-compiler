// Gramática TextMate de acm para Shiki: resalta los bloques ```acm de la landing y de la documentación.
const commands = [
  "CLICK", "PRESS", "DOUBLECLICK", "RIGHTCLICK", "MIDDLECLICK", "MOVE", "DRAG", "SCROLL", "MOUSEDOWN", "MOUSEUP",
  "WRITE", "TYPE", "TEXT", "PASTE", "KEYPRESS", "KEY", "HOTKEY", "KEYDOWN", "KEYUP", "TYPEDELAY",
  "DELAY", "WAIT", "SLEEP", "PRINT", "ECHO", "LOG", "ASSERT", "ASK", "OPEN", "EXEC", "ALERT", "NOTIFY",
  "WRITEFILE", "APPENDFILE", "DELETEFILE", "MKDIR", "CLIPSET", "FOCUS", "WAITWINDOW", "SCREENSHOT",
  "ONERROR", "FAILSAFE", "IMPORT",
  "WEBBROWSER", "WEBPROFILE", "WEBOPEN", "WEBTAB", "WEBCLICK", "WEBINPUT", "WEBGET", "WEBWAIT", "WEBSELECT",
  "WEBCHECK", "WEBSCROLL", "WEBKEY", "WEBJS", "WEBSCREENSHOT", "WEBBACK", "WEBRELOAD", "WEBCLOSE",
  "WEBWAITCLOSE", "WEBTIMEOUT",
];
const keywords = [
  "VAR", "LET", "SET", "IF", "ELSE", "ELSEIF", "ENDIF", "FOR", "IN", "TO", "STEP", "NEXT", "ENDFOR",
  "WHILE", "ENDWHILE", "LOOP", "INFINITE", "ENDLOOP", "BREAK", "CONTINUE", "RETURN", "EXIT", "FUNC",
  "ENDFUNC", "END", "AND", "OR", "NOT", "EMPTY",
];
const constants = ["TRUE", "FALSE", "NULL", "ON", "OFF", "CONTINUE", "STOP", "HEADLESS", "CHROME", "EDGE", "BRAVE"];

export const acmGrammar = {
  name: "acm",
  scopeName: "source.acm",
  aliases: ["acmx"],
  patterns: [
    { name: "comment.line.acm", match: "(^|\\s)(//|#(?![\\w{])).*$" },
    { name: "keyword.control.directive.acm", match: "^\\s*@[A-Z]+(:[A-Z]+)?" },
    {
      name: "string.quoted.double.acm",
      begin: "\"",
      end: "\"",
      patterns: [
        { name: "constant.character.escape.acm", match: "\\\\[\"$]" },
        { name: "variable.other.interpolated.acm", match: "\\$\\{[^}]*\\}" },
      ],
    },
    { name: "string.quoted.single.acm", begin: "'", end: "'" },
    { name: "support.function.command.acm", match: `(?i)\\b(${commands.join("|")})\\b` },
    { name: "keyword.control.acm", match: `(?i)\\b(${keywords.join("|")})\\b` },
    { name: "constant.language.acm", match: `\\b(${constants.join("|")})\\b` },
    { name: "constant.numeric.acm", match: "\\b\\d+(\\.\\d+)?(ms|s|min)?\\b" },
    { name: "entity.name.function.acm", match: "\\b[a-z_][a-zA-Z0-9_]*(?=\\()" },
    { name: "keyword.operator.acm", match: "->|==|!=|<=|>=|\\+=|\\+\\+|[-+*/%<>=!]" },
  ],
};
