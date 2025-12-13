// Basic Syntax Highlighter and Line Number Manager

class SwissEditor {
    constructor(editorId, highlightId, lineNumbersId) {
        this.editor = document.getElementById(editorId);
        this.highlight = document.getElementById(highlightId);
        this.lineNumbers = document.getElementById(lineNumbersId);

        this.init();
    }

    init() {
        if (!this.editor) return;

        // Bind events
        this.editor.addEventListener('input', () => this.update());
        this.editor.addEventListener('scroll', () => this.syncScroll());
        this.editor.addEventListener('keydown', (e) => this.handleKey(e));

        // Initial update
        this.update();
    }

    update() {
        const text = this.editor.value;

        // Update Line Numbers
        this.updateLineNumbers(text);

        // Update Highlighting
        this.updateHighlighting(text);
    }

    syncScroll() {
        this.highlight.scrollTop = this.editor.scrollTop;
        this.highlight.scrollLeft = this.editor.scrollLeft;
        this.lineNumbers.scrollTop = this.editor.scrollTop;
    }

    handleKey(e) {
        // Handle Tab key
        if (e.key === 'Tab') {
            e.preventDefault();
            const start = this.editor.selectionStart;
            const end = this.editor.selectionEnd;

            // Insert 2 spaces (or 4)
            this.editor.value = this.editor.value.substring(0, start) +
                "  " + this.editor.value.substring(end);

            this.editor.selectionStart = this.editor.selectionEnd = start + 2;
            this.update();
        }
    }

    updateLineNumbers(text) {
        const lines = text.split('\n').length;
        this.lineNumbers.innerHTML = Array(lines).fill(0).map((_, i) => i + 1).join('<br>');
    }

    updateHighlighting(text) {
        // Simple regex-based highlighter
        // Note: This is purely visual and naively regenerates the whole HTML.

        // Escape HTML first
        // We do this by replacing special chars.
        // Note: This naive approach complicates tokenizing if we have "&amp;" in the text.
        // Better to tokenize original text then escape the parts.

        // Let's go line by line
        const lines = text.split('\n');
        const highlightedLines = lines.map(line => this.highlightLine(line));

        // Add a trailing newline if the text ends with one, to ensure the pre block grows
        if (text.endsWith('\n')) {
            highlightedLines.push('');
        }

        this.highlight.innerHTML = highlightedLines.join('<br>');
    }

    highlightLine(line) {
        if (!line) return '';

        // We need to parse the line into tokens: Strings, Comments, Keywords, Numbers, Others.
        // A simple state machine or robust regex is needed.
        // To handle "String with ' inside", we must match strings first or in parallel.

        // We will build chunks of (text, type)
        // Types: 'string', 'comment', 'keyword', 'number', 'plain'

        // Regex for tokens:
        // 1. String: "..." (non-greedy)
        // 2. Comment: '... or REM ... (until end of line)
        // 3. Number: $Hex, %Bin, Digits
        // 4. Keyword: \bWORD\b
        // 5. Operator/Punctuation

        const keywords = [
            'REM', 'BEGIN', 'END', 'NEXT', 'WEND', 'IF', 'THEN', 'ELSE',
            'SUB', 'INTERRUPT', 'ASM', 'ON', 'AS', 'DO', 'WHILE', 'FOR',
            'TO', 'STEP', 'LOOP', 'CONST', 'DIM', 'BYTE', 'WORD', 'BOOL',
            'PEEK', 'POKE', 'PRINT', 'RETURN', 'CALL', 'AND', 'OR', 'NOT',
            'LET'
        ];
        const keywordSet = new Set(keywords);

        // Regex explanation:
        // Group 1: String ("...")
        // Group 2: Comment ('...)
        // Group 3: Hex/Bin ($..., %...)
        // Group 4: Identifier/Keyword/Number ([a-zA-Z0-9_]+)
        // We process match by match.
        // Note: REM is a keyword but also starts a comment. Special handling needed.

        const tokenRegex = /(".*?")|('.*)|(\$[0-9A-Fa-f]+|%[01]+)|([a-zA-Z0-9_]+)|(.)/g;

        let resultHtml = '';
        let match;

        // Since we are processing line by line, comments end at end of string (which is end of line).

        while ((match = tokenRegex.exec(line)) !== null) {
            const [full, str, comment, number, word, other] = match;

            if (str) {
                resultHtml += `<span class="hl-string">${this.escapeHtml(str)}</span>`;
            } else if (comment) {
                resultHtml += `<span class="hl-comment">${this.escapeHtml(comment)}</span>`;
                // Once we hit a comment, the rest of the line is part of it (Regex '.* handles this)
            } else if (number) {
                 resultHtml += `<span class="hl-number">${this.escapeHtml(number)}</span>`;
            } else if (word) {
                // Check if keyword
                const upper = word.toUpperCase();
                if (upper === 'REM') {
                    // REM is a special case: it is a keyword that starts a comment.
                    // But our regex for comment only handled '
                    // So we treat REM as the start of a comment for the rest of the line.
                    // But wait, the regex loop continues.
                    // We need to consume the rest of the line if it is REM.

                    // Actually, if we match 'REM', we should mark it as comment start?
                    // Or keyword, then the rest is comment?
                    // Standard BASIC: REM is a statement.

                    // Let's look ahead.
                    // If we found 'REM', we treat it as a comment start.
                    // But we only matched "REM".
                    // The rest of the line is not consumed by this match.

                    // Easiest fix: modify regex to include REM.* as a comment group?
                    // But REM must be a whole word.

                    // If we found a word 'REM', we highlight it as comment (or keyword),
                    // and then we manually consume the rest of the line as comment.

                     resultHtml += `<span class="hl-comment">${this.escapeHtml(word)}</span>`;
                     // Consume rest of line
                     const rest = line.substring(tokenRegex.lastIndex);
                     resultHtml += `<span class="hl-comment">${this.escapeHtml(rest)}</span>`;
                     break; // Stop processing this line
                } else if (keywordSet.has(upper)) {
                     resultHtml += `<span class="hl-keyword">${this.escapeHtml(word)}</span>`;
                } else if (/^[0-9]+$/.test(word)) {
                     // Plain integer
                     resultHtml += `<span class="hl-number">${this.escapeHtml(word)}</span>`;
                } else {
                     // Identifier
                     resultHtml += this.escapeHtml(word);
                }
            } else if (other) {
                resultHtml += this.escapeHtml(other);
            }
        }

        return resultHtml;
    }

    escapeHtml(unsafe) {
        return unsafe
            .replace(/&/g, "&amp;")
            .replace(/</g, "&lt;")
            .replace(/>/g, "&gt;")
            .replace(/"/g, "&quot;")
            .replace(/'/g, "&#039;");
    }
}

// Initialize on load
document.addEventListener('DOMContentLoaded', () => {
    window.editor = new SwissEditor('code-editor', 'code-highlight', 'line-numbers');

    // Set some default text if empty
    const editor = document.getElementById('code-editor');
    if(editor && !editor.value) {
        editor.value = "CONST BG_COLOR = $0F\n\nSUB Main()\n  ' Set Background Color\n  POKE($2006, $3F)\n  POKE($2006, $00)\n  POKE($2007, BG_COLOR)\nEND SUB";
        window.editor.update();
    }
});
