export type TextMarkType =
  | 'bold'
  | 'italic'
  | 'strike'
  | 'code'
  | 'link'
  | 'mention'

export type TextMark = {
  type: TextMarkType
  attrs?: Record<string, unknown>
}

export type TextSpan = {
  text: string
  marks?: TextMark[]
}

export type RichTextBlock = {
  type:
    | 'paragraph'
    | 'heading'
    | 'blockquote'
    | 'code_block'
    | 'list_item'
    | 'bullet_item'
    | 'ordered_item'
  text: string
  content?: TextSpan[]
  attrs?: Record<string, unknown>
}

export type PayloadRichText = {
  format: 'plaintext' | 'markdown' | 'prosemirror'
  version: number
  blocks: RichTextBlock[]
}
