import fs from 'fs';
import path from 'path';

import Footer from '../components/Footer';

export const revalidate = 60; 

export default function Page() {
    const p = path.resolve(process.cwd(), 'app', 'static'); 
    const posts = fs.readdirSync(p);

    // TODO: Populate metadata and sort by created

    return (
        <div className="text-center">
            <h1 className="text-3xl bold m-5"> Blog Posts </h1>
            <div className="grid gap-4 grid-cols-1 bold underline ">
                {posts.map((post, i) => <div key={i} className="box-content border-2 rounded-md p-3"><a href={"./posts/" + post}>{post}</a></div>)}
            </div>
            <Footer />
        </div>
    )
}