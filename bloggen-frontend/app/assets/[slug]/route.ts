// Create a route to dynamically getting a file from folder /public/images/avatar
//  app/api/public/[[...slug]]/route.ts

// I stole this code from a wonderful gentleman
// https://github.com/vercel/next.js/discussions/18005#discussioncomment-11000002

import { promises as fs, existsSync } from 'fs'
import { NextResponse } from 'next/server'
import path from 'path'

const publicFolder = path.resolve('public', 'assets')  //   /app/public (on docker)  and  ./public (on local)

export async function GET(req: Request, { params }: { params: { slug?: string } }) {
  try {
    const slug = params.slug // ["images", "avatar", "1234567.jpg"]
    const filePath = slug ? path.join(publicFolder,slug) : null  //  /public/images/avatar/1234567.jpg
    if (!filePath) throw new Error()
    
    if (!existsSync(filePath)) throw new Error("FILE DOESN'T EXIST")

    const fileContent = await fs.readFile(filePath)

    return new NextResponse(fileContent)
  } catch (err) {
    return new NextResponse(null);
  }
}