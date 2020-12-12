pub fn print_start() {
  println!(
    "
このソフトウェアは開成学園文化祭準備員会電気係で使用した、貸出品の管理ソフトウェアです。
作成は2021年1月に創立149周年記念開成祭を開催した準備員会電気係サブチーフが行いました。
使用についての保証は一切ありません。
リポジトリ：https://github.com/puripuri2100/dlm
  "
  )
}

pub fn print_restart() {
  println!("\n操作を行ってください\n操作方法がわからない場合は help と入力してください\n>")
}

pub fn print_help() {
  println!(
    "
このソフトウェア上で使うことのできるコマンドとその役割は以下の通りです。

  help      : 入力できる内容と役割（これです）を表示します

  exit      : 終了します

  lend      : 'lend <貸出品の番号1> <貸出品の番号2> .. <貸出品の番号n> <貸出先の番号>' で貸出を登録します

  l         : 'lend' の省略形です
              使い方は'lend'と変わりません

  return    : 'return <返却品の番号1> <返却品の番号2> .. <返却品の番号n> <返却元の番号>' で返却を登録します

  r         : 'return' の省略形です
              使い方は'return'と変わりません

  edit      : 'edit <編集対象に付けられた通し番号> <編集後の品名の番号> <編集後の貸出先の番号>'
              で以前に行った操作を改変できます
              'remove'と'edit'で行った操作を編集することは出来ません

  remove    : 'remove <編集対象に付けられた通し番号>'
              で以前に行った操作を無かったことにできます

  show      : 現在貸し出されているものと貸出先を表示します
              品名と貸出先の番号は実行時に与えたJSONファイルに基づいて変換されます

  all       : 全ての操作を表示します

  check     : 貸出と返却が食い違っているものが無いかをチェックします

  history   : 'history' 単体では直近10件の入力を表示します
              'history <n>' と、数字を与えるとその分だけ直近の入力を表示します
"
  )
}

pub fn print_not_found_command_name(name: String) {
  println!(
    "
  {}というコマンド名は見つかりませんでした。
  使うことのできるコマンド名は help を見てください。
  ",
    name
  )
}

pub fn print_missing_argument() {
  print!(
    "
  引数を間違えています。
  helpを入力して使い方を確認してください
"
  );
}

pub fn print_history(command_history: &[String], range: usize) {
  let vec_len = command_history.len();
  if vec_len <= range {
    for (i, item) in command_history.iter().enumerate().take(vec_len) {
      println!("{}: {}", i + 1, item)
    }
  } else {
    for (i, item) in command_history
      .iter()
      .enumerate()
      .take(vec_len)
      .skip(vec_len - range)
    {
      println!("{}: {}", i, item)
    }
  }
}

pub fn print_lend_success(product_num: &str, destination_num: &String, lend_num: &isize) {
  println!(
    "({}): {}を{}に貸し出しました",
    lend_num, product_num, destination_num
  );
}

pub fn print_return_success(product_num: &str, destination_num: &String, lend_num: &isize) {
  println!(
    "({}): {}が{}から返却されました",
    lend_num, product_num, destination_num
  );
}

pub fn print_edit_success(
  num: &isize,
  new_product_num: &str,
  new_destination_num: &String,
  lend_num: &isize,
) {
  println!(
    "({}): 操作番号{}の品名を「{}」に、貸出先を「{}」に書き換えました",
    lend_num, num, new_product_num, new_destination_num
  );
}

pub fn print_remove_success(num: &isize, lend_num: &isize) {
  println!("({}): 操作番号{}を削除しました", lend_num, num);
}
