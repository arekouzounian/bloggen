
export default function MetaHolder({ data }) {
    const d = new Date(data.LastChanged * 1000); 
    const months = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec']; 
    const month = months[d.getMonth()];
    const author = data.Author

    return (
        <div className="italic opacity-70 text-xs text-center">
            {author && <p>Author: {author}</p>}
            {month && <p>Last Changed: {month} {d.getDate()}, {d.getFullYear()}</p>}
        </div>
    )
}