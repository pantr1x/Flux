// Snippet Pack – hotové kúsky kódu pre Rust, Go, Java, C++, C#, JavaScript, TypeScript a Perl (napíš skratku a stlač Tab).
const S = {
  rust: [
    ['fn', 'function', 'fn ${1:name}(${2}) -> ${3:()} {\n\t$0\n}'],
    ['main', 'main function', 'fn main() {\n\t$0\n}'],
    ['pln', 'println!', 'println!("${1}{}", ${2});'],
    ['struct', 'struct with derive', '#[derive(Debug, Clone)]\nstruct ${1:Name} {\n\t${2:field}: ${3:i32},\n}'],
    ['impl', 'impl block', 'impl ${1:Name} {\n\tfn new(${2}) -> Self {\n\t\tSelf { $0 }\n\t}\n}'],
    ['match', 'match', 'match ${1:value} {\n\t${2:pattern} => ${3},\n\t_ => ${0},\n}'],
    ['for', 'for loop', 'for ${1:item} in ${2:iter} {\n\t$0\n}'],
    ['test', 'test module', '#[cfg(test)]\nmod tests {\n\tuse super::*;\n\n\t#[test]\n\tfn ${1:it_works}() {\n\t\tassert_eq!(${2:2 + 2}, ${3:4});\n\t}\n}'],
    ['input', 'read a line from the user', 'let mut ${1:input} = String::new();\nstd::io::stdin().read_line(&mut ${1:input}).expect("could not read");\nlet ${1:input} = ${1:input}.trim();'],
  ],
  go: [
    ['main', 'package main', 'package main\n\nimport "fmt"\n\nfunc main() {\n\t$0\n}'],
    ['fn', 'function', 'func ${1:name}(${2}) ${3:error} {\n\t$0\n}'],
    ['iferr', 'if err != nil', 'if err != nil {\n\treturn ${1:err}\n}'],
    ['pl', 'fmt.Println', 'fmt.Println(${1})'],
    ['for', 'for range', 'for ${1:i}, ${2:v} := range ${3:list} {\n\t$0\n}'],
    ['struct', 'struct', 'type ${1:Name} struct {\n\t${2:Field} ${3:string}\n}'],
  ],
  java: [
    ['main', 'public static void main', 'public static void main(String[] args) {\n\t$0\n}'],
    ['sout', 'System.out.println', 'System.out.println(${1});'],
    ['class', 'class', 'public class ${1:Name} {\n\t$0\n}'],
    ['fori', 'for loop', 'for (int ${1:i} = 0; ${1:i} < ${2:n}; ${1:i}++) {\n\t$0\n}'],
    ['foreach', 'for each', 'for (${1:String} ${2:item} : ${3:list}) {\n\t$0\n}'],
    ['scan', 'read input (Scanner)', 'java.util.Scanner ${1:in} = new java.util.Scanner(System.in);\nString ${2:line} = ${1:in}.nextLine();'],
    ['try', 'try / catch', 'try {\n\t$0\n} catch (${1:Exception} e) {\n\te.printStackTrace();\n}'],
  ],
  cpp: [
    ['main', 'int main', '#include <iostream>\n\nint main() {\n\t$0\n\treturn 0;\n}'],
    ['cout', 'std::cout', 'std::cout << ${1} << std::endl;'],
    ['cin', 'std::cin', 'std::cin >> ${1};'],
    ['fori', 'for loop', 'for (int ${1:i} = 0; ${1:i} < ${2:n}; ++${1:i}) {\n\t$0\n}'],
    ['vec', 'std::vector', 'std::vector<${1:int}> ${2:v};'],
    ['class', 'class', 'class ${1:Name} {\npublic:\n\t${1:Name}() {}\n\t$0\n};'],
  ],
  csharp: [
    ['cw', 'Console.WriteLine', 'Console.WriteLine(${1});'],
    ['rl', 'Console.ReadLine', 'string ${1:line} = Console.ReadLine();'],
    ['class', 'class', 'public class ${1:Name}\n{\n\t$0\n}'],
    ['prop', 'property', 'public ${1:int} ${2:Name} { get; set; }'],
    ['foreach', 'foreach', 'foreach (var ${1:item} in ${2:list})\n{\n\t$0\n}'],
  ],
  typescript: [
    ['fn', 'typed function', 'function ${1:name}(${2:value}: ${3:string}): ${4:void} {\n\t$0\n}'],
    ['iface', 'interface', 'interface ${1:Name} {\n\t${2:id}: ${3:number};\n}'],
    ['type', 'type alias', 'type ${1:Name} = ${2:string | number};'],
    ['class', 'class', 'class ${1:Name} {\n\tconstructor(private ${2:value}: ${3:string}) {}\n\n\t$0\n}'],
    ['afn', 'async function', 'async function ${1:load}(): Promise<${2:void}> {\n\t$0\n}'],
    ['log', 'console.log', 'console.log(${1});'],
  ],
  perl: [
    ['strict', 'use strict / warnings', 'use strict;\nuse warnings;\n$0'],
    ['sub', 'subroutine', 'sub ${1:name} {\n\tmy (${2:\\$arg}) = @_;\n\t$0\n}'],
    ['for', 'foreach loop', 'foreach my \\$${1:item} (@${2:list}) {\n\t$0\n}'],
    ['input', 'read a line from the user', 'print "${1:Your name: }";\nmy \\$${2:answer} = <STDIN>;\nchomp \\$${2:answer};'],
    ['say', 'print a line', 'print "${1}\\n";'],
  ],
  javascript: [
    ['clg', 'console.log', 'console.log(${1});'],
    ['af', 'arrow function', 'const ${1:name} = (${2}) => {\n\t$0\n};'],
    ['qs', 'querySelector', "document.querySelector('${1:#id}')"],
    ['ael', 'addEventListener', "${1:el}.addEventListener('${2:click}', (${3:e}) => {\n\t$0\n});"],
    ['fetchj', 'fetch JSON', "const ${1:data} = await fetch('${2:url}').then((r) => r.json());"],
    ['timeout', 'setTimeout', 'setTimeout(() => {\n\t$0\n}, ${1:1000});'],
  ],
};

export function activate(flux) {
  const { monaco } = flux;
  for (const [lang, list] of Object.entries(S)) {
    flux.own(
      monaco.languages.registerCompletionItemProvider(lang, {
        provideCompletionItems(model, position) {
          const word = model.getWordUntilPosition(position);
          const range = new monaco.Range(position.lineNumber, word.startColumn, position.lineNumber, word.endColumn);
          return {
            suggestions: list.map(([prefix, detail, body]) => ({
              label: prefix,
              kind: monaco.languages.CompletionItemKind.Snippet,
              detail: `${detail} (Snippet Pack)`,
              documentation: { value: '```\n' + body.replace(/\$\{\d+:?([^}]*)\}/g, '$1').replace(/\$\d/g, '') + '\n```' },
              insertText: body,
              insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
              range,
            })),
          };
        },
      }),
    );
  }
}
