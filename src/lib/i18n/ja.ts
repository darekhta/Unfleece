// Japanese bundle — full translation of the canonical English source (en.ts).
// Falls back to English for anything missing.
import type { LocaleBundle } from './types.js';

export const ja: LocaleBundle = {
  meta: { code: 'ja', htmlLang: 'ja', autonym: '日本語', englishName: 'Japanese', dir: 'ltr' },
  categories: {
    organize: { label: '整理', descriptor: '並べ替え・結合・分割・回転' },
    convert: { label: '変換', descriptor: 'PDF・画像・テキストの間で変換' },
    edit: { label: '編集', descriptor: 'ページにスタンプ・マークアップ・署名' },
    optimize: { label: '最適化', descriptor: 'ファイルを小さく' },
    forms: { label: 'フォーム', descriptor: 'フォーム欄を検出して入力' },
    security: { label: 'セキュリティとプライバシー', descriptor: '共有したくない情報を取り除く' },
  },
  layout: {
    skip: 'コンテンツへスキップ',
    homeAria: 'Unfleece ホーム',
    navAria: 'メインナビゲーション',
    allTools: 'すべてのツール',
    howItWorks: '使い方',
    privacy: 'プライバシー',
    footerBlurb: '無料でプライベートな PDF ツール。ファイルはブラウザの外に出ません。',
    tools: 'ツール',
    project: 'プロジェクト',
    trust: '信頼性',
    github: 'GitHub ↗',
    license: 'AGPL-3.0',
    reportIssue: '問題を報告 ↗',
    noUpload: 'ファイルのアップロードなし',
    noAccount: 'アカウント不要',
    noTracking: 'トラッキングなし',
    language: '言語',
  },
  toolPage: {
    breadcrumbHome: 'ホーム',
    breadcrumbTools: 'ツール',
    privateWorkspace: 'プライベートな作業スペース',
    nothingUploaded: 'ファイルはアップロードされません',
    howEyebrow: '使い方',
    howHeading: (toolName) => `${toolName}の使い方`,
    steps: [
      { title: 'ファイルを追加', body: 'ドラッグするか、クリックして選択。ファイルはあなたのデバイスに残ります。' },
      { title: 'オプションを設定', body: '設定を調整するか、そのまま無難な初期設定を使ってもOK。' },
      { title: '結果をダウンロード', body: 'ワンクリック。すべてこのブラウザの中で作られました。' },
    ],
    differenceEyebrow: '正直な違い',
    differenceHeading: 'Unfleece が fleeceware と違う理由',
    usBullets: ['ブラウザ内で動作 — ファイルのアップロードなし', '無料、アカウント不要、カード登録なし', 'オープンソース（AGPL-3.0）', 'ファイルサイズによる有料の壁なし'],
    themHeading: 'よくある fleeceware サイト',
    themBullets: ['ファイルをサーバーにアップロード', '$1 のお試し → 週あたり約 $50', '非公開で中身が見えない', '結果を課金の後ろに隠す'],
    questionsEyebrow: '質問',
    questionsHeading: 'よくある質問',
    relatedEyebrow: 'もっと使う',
    relatedHeading: '関連ツール',
    faqs: (toolName) => [
      {
        q: `${toolName}は本当に無料ですか？`,
        a: 'はい — 永久に無料、アカウント不要、あとで噛みついてくるお試し期間もありません。何もアップロードしないので、回収すべきサーバー代がそもそも存在しないのです。',
      },
      {
        q: 'ファイルはどこかにアップロードされますか？',
        a: `いいえ。ブラウザがファイルをメモリに読み込み、${toolName}をあなたのデバイス上で実行します。ネットワークタブを開いて見てみてください — アップロードのリクエストはゼロのはずです。`,
      },
      {
        q: 'ファイルサイズやページ数の上限はありますか？',
        a: '厳密な上限はありません。唯一の天井はブラウザのメモリなので、非常に大きなファイルは遅くなることがあります。それが問題になりそうなときは事前にお知らせします。',
      },
      {
        q: 'オフラインでも動きますか？',
        a: 'ページが一度読み込まれていれば、はい。Wi-Fi を切っても動作します — 何もデバイスから出ていない、何よりの証拠です。',
      },
    ],
  },
  home: {
    heroLine1: '無料でプライベートな PDF ツール。',
    heroLine2: 'ファイルはブラウザの外に出ません。',
    subhead:
      '結合、分割、変換、署名、圧縮 — すべてのツールが完全にあなたのデバイス上で動作します。永久に無料、アカウント不要、アップロードなし。オープンソースで、それを正直に公言しています。',
    browseCta: 'ツールを見る',
    howCta: '使い方',
    proof: [
      { title: '設計からしてプライベート', body: 'ファイルはローカルで処理され、決してアップロードされません。', linkLabel: 'ネットワークタブで確認できます。' },
      { title: '永久に無料', body: 'お試し期間も、サブスクも、ファイルサイズの有料の壁もありません。アップセルするものが何もないのです。' },
      { title: '高速でオープン', body: 'WebAssembly でローカル実行。完全なオープンソース（AGPL-3.0）。' },
    ],
    toolsEyebrow: 'すべてのツール',
    pickHeading: 'ツールを選ぶ — 開いて、実行して、ダウンロード。',
    pickBody: '登録なし。順番待ちなし。すべてがこのタブの中で完結します。',
    proveEyebrow: '信じなくていい — 自分で確かめて',
    proveHeading: '10秒で動き、しかもプライベートだと自分で証明できます。',
    proveBody:
      'ファイルがサーバーに触れることは一切ないので、あなたに請求するものも、漏れるものも何もありません。無料は、ただの正直な初期設定なのです。',
    proveSteps: [
      { strong: 'どれかツールを開いて', rest: '、F12 → ネットワークタブを開く。' },
      { strong: 'ファイルで実行する。', rest: ' リクエスト一覧を眺める。' },
      { strong: '何もアップロードされないのを見る。', rest: ' Wi-Fi を切っても、ちゃんと動きます。' },
    ],
  },
  tools: {
    'merge': {
      name: 'PDF を結合',
      tagline: '複数の PDF を 1 つの文書にまとめる。',
      description: '複数の PDF ファイルを、追加した順番のまま 1 つの文書に結合します。',
    },
    'split': {
      name: 'PDF を分割',
      tagline: '1 つの PDF を複数のファイルに分割。',
      description: '1 つの PDF を別々のファイルに分割します — 1 ページずつ、または指定したページ範囲ごとに。',
    },
    'extract-pages': {
      name: 'ページを抽出',
      tagline: '必要なページだけを取り出す。',
      description: '選んだページだけを含む新しい PDF を作成します。',
    },
    'remove-pages': {
      name: 'ページを削除',
      tagline: 'いらないページを取り除く。',
      description: '選んだページを削除し、残りを保持します。',
    },
    'reorder': {
      name: 'ページを並べ替え',
      tagline: 'ページを新しい順番に並べ替える。',
      description: 'PDF のページを新しい順序に並べ替えます。',
    },
    'rotate': {
      name: 'PDF を回転',
      tagline: 'ページを正しい向きに直す。',
      description: 'すべて、または選択したページを 90°・180°・270° 回転します。劣化なし。',
    },
    'n-up': {
      name: '1 枚に複数ページ（N-up）',
      tagline: '1 枚に複数ページをまとめて印刷。',
      description: '複数の元ページを 1 枚の A4 用紙に配置します — 配布資料に最適。',
    },
    'booklet': {
      name: '小冊子',
      tagline: '折り本印刷用にページを面付け。',
      description: '横向きの用紙に 2 ページずつ並べて面付けし、折ると小冊子になるようにします。',
    },
    'compare-pdf': {
      name: 'PDF を比較',
      tagline: '2 つの PDF の見た目の違いを見つける。',
      description: '2 つの PDF をローカルでレンダリングし、ページごとの視覚的な差分レポートを作成します。',
    },
    'images-to-pdf': {
      name: '画像 → PDF',
      tagline: 'JPG や PNG を PDF にする。',
      description: '1 枚以上の JPG/PNG 画像を、1 ページ 1 画像で PDF にします。',
    },
    'pdf-to-jpg': {
      name: 'PDF → JPG',
      tagline: '各ページを JPG として保存。',
      description: 'PDF の各ページを JPG 画像にレンダリングし、ZIP でまとめてダウンロードします。',
    },
    'pdf-to-png': {
      name: 'PDF → PNG',
      tagline: '各ページを PNG として保存。',
      description: 'PDF の各ページを劣化のない PNG 画像にレンダリングし、ZIP でまとめてダウンロードします。',
    },
    'pdf-to-text': {
      name: 'PDF → テキスト',
      tagline: 'PDF から選択可能なテキストを取り出す。',
      description: 'PDF から選択可能なテキストを抽出します。（スキャン/画像の PDF には抽出できるテキストがありません。）',
    },
    'pdf-to-epub': {
      name: 'PDF → EPUB',
      tagline: '選択可能な PDF テキストから EPUB を作成。',
      description: '選択可能な PDF テキストからリフロー型 EPUB を作るか、スキャンや複雑なレイアウト向けにレンダリングしたページから固定レイアウト型 EPUB を作ります。',
    },
    'pdf-to-docx': {
      name: 'Word に書き出し',
      tagline: '選択可能な PDF テキストを DOCX として保存。',
      description: '選択可能なテキストをシンプルな Word 文書に抽出します。元のレイアウトではなく、テキストを保持します。',
    },
    'pdf-to-excel': {
      name: 'Excel に書き出し',
      tagline: '選択可能な行をスプレッドシートに。',
      description: '選択可能なテキスト行を、できる限り Excel ブックに抽出します。スキャンはまず OCR が必要です。',
    },
    'pdf-to-pptx': {
      name: 'PowerPoint に書き出し',
      tagline: 'PDF のテキストをシンプルなスライドに。',
      description: '選択可能な PDF テキストを、1 ページ 1 スライドでシンプルな PowerPoint に抽出します。元のレイアウトは再現しません。',
    },
    'ocr-pdf': {
      name: 'OCR で検索可能な PDF',
      tagline: 'スキャンした PDF に選択可能なテキストを追加。',
      description: 'Tesseract OCR をローカルで実行し、各ページの上に見えない検索可能なテキスト層を追加します。英語モデルを同梱。',
    },
    'pdfa': {
      name: 'PDF/A で書き出し',
      tagline: '保存用の PDF/A-2b コピーを作成。',
      description: 'ページをローカルでレンダリングし、Rust 製の krilla エンジンで視覚的な PDF/A-2b ファイルとして再構築します。テキストは選択できなくなります。',
    },
    'image-convert': {
      name: '画像を変換',
      tagline: 'PNG・JPG・WebP・AVIF・JPEG XL や TIFF 系の入力を相互変換。',
      description: 'ブラウザ対応の画像や、TIFF・PSD・BMP・GIF・ICO などの形式を、すべてブラウザ内で PNG・JPG・WebP・AVIF・JPEG XL に変換します。',
    },
    'html-to-pdf': {
      name: 'HTML / Markdown → PDF',
      tagline: 'テキスト文書をきれいな PDF にする。',
      description: 'HTML・Markdown・プレーンテキストのファイルを、読みやすい PDF に変換します。テキスト重視で、ブラウザのレイアウトエンジンではありません。',
    },
    'page-numbers': {
      name: 'ページ番号',
      tagline: 'PDF にページ番号を入れる。',
      description: '書式と位置を自由に設定して、ページ番号を追加します。',
    },
    'bates': {
      name: 'ベイツ番号',
      tagline: '法務向けのベイツ番号を追加。',
      description: '接頭辞・桁揃えした番号・位置を指定して、各ページに連番のベイツ番号をスタンプします。',
    },
    'watermark': {
      name: '透かし',
      tagline: 'すべてのページにテキストの透かしを追加。',
      description: 'すべてのページに、斜めのテキスト透かしをスタンプします。',
    },
    'crop': {
      name: 'トリミング',
      tagline: 'ページの余白を切り取る。',
      description: 'クロップボックスを設定して、すべてのページの余白を切り取ります。',
    },
    'auto-crop': {
      name: '余白を自動トリミング',
      tagline: '白い余白を検出して切り取る。',
      description: '各ページをローカルでレンダリングし、白くない内容の範囲を検出して、余白付きでページごとのクロップボックスを設定します。',
    },
    'metadata': {
      name: 'メタデータを編集',
      tagline: 'タイトルや作成者などを変更。',
      description: '文書のメタデータ（タイトル、作成者、件名、キーワード）を表示して上書きします。',
    },
    'sign': {
      name: '署名',
      tagline: '署名を描いて配置する。',
      description: '署名を描いてページに配置します。電子的なサインであり、暗号的な署名ではありません。',
    },
    'optimize': {
      name: '最適化',
      tagline: '品質を落とさずにファイルを小さく。',
      description: '未使用のオブジェクトを取り除きストリームを圧縮して、PDF を劣化なしで小さくします — テキストは選択可能なまま。',
    },
    'compress': {
      name: '圧縮',
      tagline: '画像の多い PDF を Ghostscript で圧縮。',
      description: 'Ghostscript の pdfwrite をローカルで使い、PDF を非可逆圧縮します。Ghostscript がファイルを書き換えられる場合はテキストが選択可能なまま残り、ラスター方式のフォールバックではテキストが選択できなくなります。',
    },
    'compress-image': {
      name: '画像を圧縮',
      tagline: 'PNG・JPG・WebP・AVIF・JPEG XL や TIFF 系の入力を小さく。',
      description: 'ブラウザ対応の画像に加え、TIFF・PSD・BMP・GIF・ICO を、ローカルで再エンコードし必要に応じてリサイズします。より小さい非可逆出力には WebP・AVIF・JPEG XL・JPG を選べます。',
    },
    'fill-form': {
      name: 'フォームに入力',
      tagline: '検出したフォーム欄に入力。',
      description: 'AcroForm のフィールドを検出し、ブラウザ内で入力します。',
    },
    'flatten': {
      name: '平坦化',
      tagline: 'フォーム欄をページに焼き込む。',
      description: 'フォーム欄や注釈を、静的なページ内容に平坦化します。',
    },
    'remove-metadata': {
      name: 'メタデータを削除',
      tagline: '共有前に隠れた情報を取り除く。',
      description: '文書のメタデータ（タイトル、作成者、生成ソフトなど）を削除し、うっかり共有されないようにします。',
    },
    'protect': {
      name: 'PDF を保護',
      tagline: 'PDF を開くためのパスワードを追加。',
      description: 'ユーザーパスワードと、任意のオーナーパスワード/権限で、PDF をローカルで暗号化します。',
    },
    'unlock': {
      name: 'PDF のロック解除',
      tagline: 'パスワードがわかっていれば暗号化を解除。',
      description: '暗号化された PDF をパスワードで開き、ロックを解除したコピーをローカルに保存します。',
    },
    'sanitize': {
      name: 'PDF をサニタイズ',
      tagline: 'PDF のアクティブ要素や隠し要素を取り除く。',
      description: '共有前に、メタデータ・文書スクリプト・埋め込みファイル・ページアクション、そして任意で注釈/フォームを取り除きます。',
    },
    'redact': {
      name: 'PDF を黒塗り',
      tagline: '黒塗りボックスを焼き込む。',
      description: '選択した範囲を覆い、ページを画像のみの PDF ページとして再構築するので、隠したテキストが下に残りません。',
    },
  },
};

export default ja;
