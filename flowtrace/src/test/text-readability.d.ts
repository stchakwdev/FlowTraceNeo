// text-readability 1.1.1 ships no types; the landing copy test uses these two scores.
declare module 'text-readability' {
  const readability: {
    fleschKincaidGrade: (text: string) => number
    fleschReadingEase: (text: string) => number
  }
  export default readability
}
