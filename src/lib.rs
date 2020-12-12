use chrono::FixedOffset;
use std::cmp::Ordering;

#[derive(Debug, Clone)]
pub struct ConfigData {
  // 貸し出した機材の番号と名前の対応データ
  pub sizai: serde_json::Value,
  // 貸出先の参団の番号と名前の対応データ
  pub sandan: serde_json::Value,
  // 貸出先の参団の番号と場所の対応データ
  pub room: serde_json::Value,
}

pub fn make_config_data(
  sizai: serde_json::Value,
  sandan: serde_json::Value,
  room: serde_json::Value,
) -> ConfigData {
  ConfigData {
    sizai,
    sandan,
    room,
  }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LendType {
  // 貸出：「貸し出した品名」と「貸出先」
  Lend(String, String),
  // 返却：「返された品名」と「返却先」
  Return(String, String),
  // 編集：「編集する操作対象に付けられた通し番号」と「編集後の品名」と「編集後の貸出先」
  Edit(isize, String, String),
  // 削除：「削除する操作対象に付けられた通し番号」
  Remove(isize),
}

#[derive(Debug, Clone, PartialEq)]
pub struct LendData {
  // 操作を行った日時
  pub time: chrono::DateTime<FixedOffset>,
  // 貸出・返却・修正・削除のどれなのかを記録する
  pub lend_type: LendType,
  // 操作番号（編集や削除するときに使う）
  pub num: isize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShowLendData {
  pub time: chrono::DateTime<FixedOffset>,
  pub product_num: String,
  pub destination_num: String,
  pub num: isize,
}

// 同じtypeなら操作番号が大きい方が大きい
// 違うtypeならremoveが一番大きく、editが二番目に大きい
impl PartialOrd for LendData {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    match (self.clone().lend_type, other.clone().lend_type) {
      (LendType::Remove(_), LendType::Remove(_)) => Some(self.num.cmp(&other.num)),
      (LendType::Remove(_), _) => Some(Ordering::Greater),
      (LendType::Edit(_, _, _), LendType::Edit(_, _, _)) => Some(self.num.cmp(&other.num)),
      (LendType::Edit(_, _, _), LendType::Remove(_)) => Some(Ordering::Less),
      (LendType::Edit(_, _, _), _) => Some(Ordering::Greater),
      _ => Some(self.num.cmp(&other.num)),
    }
  }
}

impl ToString for LendData {
  fn to_string(&self) -> String {
    let time = self.time;
    let time_str = time.format("%Y/%m/%d %H:%M").to_string();
    let num = self.num;
    let lend_type = self.clone().lend_type;
    let lend_type_str = match lend_type {
      LendType::Lend(product_num, destination_num) => {
        format!("{}を{}へ貸出", product_num, destination_num)
      }
      LendType::Return(product_num, destination_num) => {
        format!("{}を{}が返却", product_num, destination_num)
      }
      LendType::Edit(num, new_product_num, new_destination_num) => {
        format!(
          "{}番目の操作の品名を\"{}\"に、相手を\"{}\"に修正する",
          num, new_product_num, new_destination_num
        )
      }
      LendType::Remove(num) => format!("{}番目の操作を無かったことにする", num),
    };
    let num_str = format!("({})", num);
    format!("{}  {}  \"{}\"", num_str, time_str, lend_type_str)
  }
}

#[test]
fn check_sort_lend_data() {
  use chrono::Utc;
  // removeが一番前に来るかの検査
  // 操作番号が後ろのやつが前に来てほしい
  let time = Utc::now().with_timezone(&FixedOffset::east(9 * 3600));
  let mut lst = vec![
    LendData {
      time: time,
      lend_type: LendType::Lend(String::new(), String::new()),
      num: 1,
    },
    LendData {
      time: time,
      lend_type: LendType::Edit(1, String::new(), String::new()),
      num: 2,
    },
    LendData {
      time: time,
      lend_type: LendType::Remove(1),
      num: 3,
    },
    LendData {
      time: time,
      lend_type: LendType::Lend(String::new(), String::new()),
      num: 4,
    },
  ];
  lst.sort_by(|a, b| b.partial_cmp(a).unwrap());
  let lst2 = vec![
    LendData {
      time: time,
      lend_type: LendType::Remove(1),
      num: 3,
    },
    LendData {
      time: time,
      lend_type: LendType::Edit(1, String::new(), String::new()),
      num: 2,
    },
    LendData {
      time: time,
      lend_type: LendType::Lend(String::new(), String::new()),
      num: 4,
    },
    LendData {
      time: time,
      lend_type: LendType::Lend(String::new(), String::new()),
      num: 1,
    },
  ];
  assert_eq!(lst, lst2);
}

// データからremoveやeditを反映させ、綺麗なデータを作る
pub fn organize_lend_data(lend_data_lst: &[LendData]) -> Vec<LendData> {
  let mut sort_lend_data_lst = lend_data_lst.to_owned();
  // 大きい順に並び変えることで、removeとeditを先にし、最初に処理を行う
  sort_lend_data_lst.sort_by(|a, b| b.partial_cmp(a).unwrap());
  loop {
    // 先頭がRemoveでもEditでも、自分自身を削除して
    // sort_lend_data_lstを更新するので、常に先頭を取り続けて良い
    let next = sort_lend_data_lst.get(0);
    match next {
      None => break,
      Some(lend_data) => {
        let next_id = lend_data.clone().num;
        match lend_data.clone().lend_type {
          LendType::Remove(num) => {
            // 番号が一致するデータ以外のリストを作成し、上書きする
            let new_sort_lend_data_lst: Vec<LendData> = sort_lend_data_lst
              .iter()
              // remove自身も削除する
              .filter(|data| !(data.num == num || data.num == next_id))
              .cloned()
              .collect();
            sort_lend_data_lst = new_sort_lend_data_lst;
          }
          LendType::Edit(num, new_product_num, new_destination_num_opt) => {
            // 番号が一致するデータを上書きする
            let new_sort_lend_data_lst: Vec<LendData> = sort_lend_data_lst
              .iter()
              .map(|data| {
                if data.num == num {
                  match data.lend_type {
                    LendType::Lend(_, _) => LendData {
                      time: data.time,
                      num: data.num,
                      lend_type: LendType::Lend(
                        new_product_num.clone(),
                        new_destination_num_opt.clone(),
                      ),
                    },
                    LendType::Return(_, _) => LendData {
                      time: data.time,
                      num: data.num,
                      lend_type: LendType::Return(
                        new_product_num.clone(),
                        new_destination_num_opt.clone(),
                      ),
                    },
                    LendType::Edit(_, _, _) => data.clone(),
                    LendType::Remove(_) => data.clone(),
                  }
                } else {
                  data.clone()
                }
              })
              // 自分自身を削除する
              .filter(|data| data.num != next_id)
              .collect();
            sort_lend_data_lst = new_sort_lend_data_lst;
          }
          // RemoveとEditが先に並んでいるはずなので、どちらかに到達したらその時点で終了しても大丈夫
          LendType::Lend(_, _) => break,
          LendType::Return(_, _) => break,
        }
      }
    }
  }
  // 操作番号が大きい（後に行った）ものが前になるように並べていたので、反転して通常に戻す
  sort_lend_data_lst.reverse();
  sort_lend_data_lst
}

// 操作番号から操作番号の種類を取り出す
pub fn get_lend_data(lend_data: &[LendData], n: isize) -> Option<LendData> {
  lend_data.iter().find(|data| data.num == n).cloned()
}

// 'show'コマンドで表示する内容を作成する
// 操作番号   時刻               貸出品                       貸出先（団体名）（場所）
//    (1) :   2020/11/23 17:40   0001（内リール1）            0（電気係）（第二会議室）
// という内容
fn show_lend_data_to_string(show_lend_data: &ShowLendData, config_data: &ConfigData) -> String {
  let time = show_lend_data.time;
  let time_str = time.format("%Y/%m/%d %H:%M").to_string();
  let product_num = &show_lend_data.product_num;
  let product_name = match config_data.sizai[product_num].as_str() {
    None => String::new(),
    Some(s) => format!("（{}）", s),
  };
  let destination_num = &show_lend_data.destination_num;
  let destination_str = {
    match (
      config_data.sandan[destination_num].as_str(),
      config_data.room[destination_num].as_str(),
    ) {
      (Some(sandan), Some(room)) => format!("{}（{}）（{}）", destination_num, sandan, room),
      (Some(sandan), None) => format!("{}（{}）", destination_num, sandan),
      (None, Some(room)) => format!("{}（？）（{}）", destination_num, room),
      (None, None) => destination_num.to_string(),
    }
  };
  let num = show_lend_data.num;
  let num_str = format!("({}):", num);
  let product_str = format!("{}{}", product_num, product_name);
  format!(
    "{num:>8}   {time:<16}   {product:<030}   {destination_str:<030}\n",
    num = num_str,
    time = time_str,
    product = product_str,
    destination_str = destination_str
  )
}

// 貸出中の品を表示するための文字列を作る
pub fn make_lend_data_str(lend_data_lst: Vec<LendData>, config_data: ConfigData) -> String {
  let mut lend_data_lst = organize_lend_data(&lend_data_lst);
  // 操作番号が小さい方が最初になるように並び替える
  lend_data_lst.sort_by(|a, b| a.num.cmp(&b.num));
  // 貸したものを登録し、返却があったら削除する
  // (時間, 品名番号, Option<貸出先番号>, 操作番号)
  let mut lend_vec: Vec<ShowLendData> = Vec::new();
  for lend_data in lend_data_lst.iter() {
    let lend_type = &lend_data.lend_type;
    match lend_type {
      LendType::Lend(product_num, destination_num) => lend_vec.push(ShowLendData {
        time: lend_data.time,
        product_num: product_num.clone(),
        destination_num: destination_num.clone(),
        num: lend_data.num,
      }),
      // 貸出品の番号が一致していたら削除
      LendType::Return(product_num, _) => {
        lend_vec = lend_vec
          .iter()
          .filter(|data| &data.product_num != product_num)
          .cloned()
          .collect();
      }
      // editとremoveは反映し終わっているはずなので考慮しない
      _ => (),
    }
  }
  lend_vec
    .iter()
    .map(|show_lend_data| show_lend_data_to_string(show_lend_data, &config_data))
    .collect()
}

// 引数をデータ構造に落とす
#[derive(Debug, Clone)]
pub enum DlmArg {
  Null,
  Help,
  Exit,
  NotFoundCommandName(String),
  MissingArgument,
  History(usize),
  Show,
  AllPrint,
  Check,
  Lend(Vec<String>, String),
  Return(Vec<String>, String),
  Edit(isize, String, String),
  Remove(isize),
}

// 大文字小文字を考慮するのが面倒なので、アルファベットに関しては小文字化して評価する
pub fn parse_arg(arg: Vec<&str>) -> DlmArg {
  if arg.is_empty() {
    DlmArg::Null
  } else {
    let arg_command_name: &str = &arg[0].to_owned().to_ascii_lowercase();
    match arg_command_name {
      "exit" => DlmArg::Exit,
      "help" => DlmArg::Help,
      "history" => match arg.get(1) {
        None => DlmArg::History(10),
        Some(s) => {
          let n = s.parse().unwrap();
          DlmArg::History(n)
        }
      },
      "show" => DlmArg::Show,
      "all" => DlmArg::AllPrint,
      "check" => DlmArg::Check,
      "lend" | "l" => {
        // <貸出品の番号1> <貸出品の番号2> .. <貸出品の番号n> <貸出先の番号>
        match arg.get(1) {
          None => DlmArg::MissingArgument,
          Some(_) => {
            let mut v = Vec::new();
            let len = arg.len();
            for i in 1..(len - 1) {
              v.push(arg[i].to_string())
            }
            match arg.iter().last() {
              None => DlmArg::MissingArgument,
              Some(s) => DlmArg::Lend(v, s.to_string()),
            }
          }
        }
      }
      "return" | "r" => {
        // <返却品の番号1> <返却品の番号2> .. <返却品の番号n> <返却元の番号>
        let mut v = Vec::new();
        let len = arg.len();
        for i in 1..(len - 1) {
          v.push(arg[i].to_string())
        }
        DlmArg::Return(v, arg[len].to_string())
      }
      "edit" => {
        // <編集対象に付けられた通し番号> <編集後の品名の番号> <編集後の貸出先の番号>
        match arg.get(1) {
          None => DlmArg::MissingArgument,
          Some(s1) => match s1.parse() {
            Err(_) => DlmArg::MissingArgument,
            Ok(i) => match arg.get(2) {
              None => DlmArg::MissingArgument,
              Some(s2) => match arg.get(3) {
                None => DlmArg::MissingArgument,
                Some(s3) => DlmArg::Edit(i, s2.to_string(), s3.to_string()),
              },
            },
          },
        }
      }
      "remove" => match arg.get(1) {
        None => DlmArg::MissingArgument,
        Some(s) => match s.parse() {
          Err(_) => DlmArg::MissingArgument,
          Ok(i) => DlmArg::Remove(i),
        },
      },
      // コメント扱い
      "#" => DlmArg::Null,
      name => DlmArg::NotFoundCommandName(name.to_owned()),
    }
  }
}
