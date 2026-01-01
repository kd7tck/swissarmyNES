import re

filepath = "src/compiler/codegen.rs"
with open(filepath, "r") as f:
    content = f.read()

# Replace multi-line calls:
# self.output
#    .push(...)
# to
# self
#    .emit(...)
# We match "self.output" followed by whitespace/newline followed by ".push"
# We replace "self.output" with "self" and ".push" with ".emit"
def replace_multiline(match):
    s = match.group(0)
    s = s.replace("self.output", "self")
    s = s.replace(".push", ".emit")
    return s

content = re.sub(r"self\.output\s*\n\s*\.push", replace_multiline, content)

# Replace single-line calls:
content = content.replace("self.output.push", "self.emit")

# Replace clear calls if any (though generate handles it manually in my patch)
content = content.replace("self.output.clear()", "self.bank_outputs.clear()")

with open(filepath, "w") as f:
    f.write(content)
